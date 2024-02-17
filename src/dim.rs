use std::collections::HashMap;

use log::{debug, error};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_simple,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            globals::GlobalList,
            protocol::{
                wl_buffer::{self, WlBuffer},
                wl_keyboard,
                wl_output::WlOutput,
                wl_pointer,
            },
            Connection, Dispatch, QueueHandle,
        },
        protocols::wp::{
            single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::{
                self, WpSinglePixelBufferManagerV1,
            },
            viewporter::client::{
                wp_viewport::{self, WpViewport},
                wp_viewporter::{self, WpViewporter},
            },
        },
    },
    registry::{ProvidesRegistryState, RegistryState, SimpleGlobal},
    registry_handlers,
    seat::{
        keyboard::KeyboardHandler,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface},
        WaylandSurface,
    },
};

use crate::INIT_SIZE;

pub struct DimData {
    compositor: CompositorState,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    layer_shell: LayerShell,
    pixel_buffer_mgr: SimpleGlobal<WpSinglePixelBufferManagerV1, 1>,
    viewporter: SimpleGlobal<WpViewporter, 1>,

    views: HashMap<WlOutput, DimView>,

    exit: bool,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
}

struct DimView {
    first_configure: bool,
    width: u32,
    height: u32,
    buffer: WlBuffer,
    viewport: WpViewport,
    layer: LayerSurface,
}

impl DimData {
    pub fn new(
        compositor: CompositorState,
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        layer_shell: LayerShell,
    ) -> Self {
        Self {
            compositor,
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, qh),
            output_state: OutputState::new(globals, qh),
            layer_shell,
            pixel_buffer_mgr: SimpleGlobal::<WpSinglePixelBufferManagerV1, 1>::bind(globals, qh)
                .expect("wp_single_pixel_buffer_manager_v1 not available!"),
            viewporter: SimpleGlobal::<wp_viewporter::WpViewporter, 1>::bind(globals, qh)
                .expect("wp_viewporter not available"),

            views: HashMap::new(),

            exit: false,
            keyboard: None,
            keyboard_focus: true,
            pointer: None,
        }
    }

    pub fn should_exit(&self) -> bool {
        self.exit
    }

    fn create_view(&self, qh: &QueueHandle<Self>, output: &WlOutput) -> DimView {
        let surface = self.compositor.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("dim_layer"),
            Some(output),
        );

        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.set_size(INIT_SIZE, INIT_SIZE);

        layer.commit();

        let viewport = self
            .viewporter
            .get()
            .expect("wp_viewporter failed")
            .get_viewport(layer.wl_surface(), qh, ());

        DimView::new(
            self.pixel_buffer_mgr.get().expect("failed to get buffer"),
            qh,
            viewport,
            layer,
        )
    }
}

impl DimView {
    fn new(
        mgr: &WpSinglePixelBufferManagerV1,
        qh: &QueueHandle<DimData>,
        viewport: WpViewport,
        layer: LayerSurface,
    ) -> Self {
        Self {
            first_configure: true,
            width: INIT_SIZE,
            height: INIT_SIZE,
            buffer: mgr.create_u32_rgba_buffer(0, 0, 0, u32::MAX, qh, ()),
            viewport,
            layer,
        }
    }

    fn draw(&mut self, qh: &QueueHandle<DimData>) {
        if !self.first_configure {
            // we only need to draw once as it is a static color
            return;
        }

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());
        self.layer.wl_surface().attach(Some(&self.buffer), 0, 0);
        self.layer.commit();

        debug!("Drawn");
    }
}

impl LayerShellHandler for DimData {
    fn closed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
    ) {
        debug!("Closed");
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let Some(view) = self.views.values_mut().find(|view| &view.layer == layer) else {
            error!("Configuring layer not belonging to self.views?");
            return;
        };

        (view.width, view.height) = configure.new_size;

        view.viewport
            .set_destination(view.width as _, view.height as _);

        if view.first_configure {
            view.draw(qh);
            view.first_configure = false;
        }
    }
}

impl CompositorHandler for DimData {
    fn scale_factor_changed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _new_transform: smithay_client_toolkit::reexports::client::protocol::wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        for view in self.views.values_mut() {
            view.draw(qh);
        }
    }
}

impl OutputHandler for DimData {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        let view = self.create_view(qh, &output);
        self.views.insert(output, view);
    }

    fn update_output(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.views.remove(&output);
        let view = self.create_view(qh, &output);
        self.views.insert(output, view);
    }

    fn output_destroyed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.views.remove(&output);
    }
}

impl SeatHandler for DimData {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _seat: smithay_client_toolkit::reexports::client::protocol::wl_seat::WlSeat,
    ) {
    }

    fn new_capability(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        seat: smithay_client_toolkit::reexports::client::protocol::wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => {
                self.keyboard = Some(
                    self.seat_state
                        .get_keyboard(qh, &seat, None)
                        .expect("Failed to get keyboard"),
                )
            }
            Capability::Pointer => {
                self.pointer = Some(
                    self.seat_state
                        .get_pointer(qh, &seat)
                        .expect("Failed to get pointer"),
                )
            }
            Capability::Touch => todo!(),
            _ => debug!("Unknown capability found: {capability}"),
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _seat: smithay_client_toolkit::reexports::client::protocol::wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => self
                .keyboard
                .take()
                .expect("Failed to remove keyboard!")
                .release(),
            Capability::Pointer => self
                .pointer
                .take()
                .expect("Failed to remove pointer!")
                .release(),
            Capability::Touch => todo!(),
            _ => todo!(),
        }
    }

    fn remove_seat(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _seat: smithay_client_toolkit::reexports::client::protocol::wl_seat::WlSeat,
    ) {
    }
}

impl KeyboardHandler for DimData {
    fn enter(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
        if self
            .views
            .values()
            .any(|view| view.layer.wl_surface() == surface)
        {
            debug!("Gained keyboard focus");
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        if self
            .views
            .values()
            .any(|view| view.layer.wl_surface() == surface)
        {
            debug!("Lost keyboard focus");
            self.keyboard_focus = false;
        }
    }

    fn press_key(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        debug!("Key pressed");
        self.exit = true;
    }

    fn release_key(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        debug!("Key released");
    }

    fn update_modifiers(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) {
        debug!("Modifiers updated");
    }
}

impl PointerHandler for DimData {
    fn pointer_frame(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for e in events {
            match e.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Leave { .. } => {
                    debug!("Mouse left or entered!")
                }
                _ => self.exit = true,
            }
        }
    }
}

delegate_compositor!(DimData);
delegate_layer!(DimData);
delegate_registry!(DimData);
delegate_pointer!(DimData);
delegate_keyboard!(DimData);
delegate_output!(DimData);
delegate_seat!(DimData);
delegate_simple!(DimData, WpViewporter, 1);

impl ProvidesRegistryState for DimData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

impl Dispatch<WpViewport, ()> for DimData {
    fn event(
        _: &mut Self,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        unreachable!("wp_single_pixel_buffer_manager_v1::Event is empty in version 1")
    }
}

impl Dispatch<WpSinglePixelBufferManagerV1, ()> for DimData {
    fn event(
        _: &mut Self,
        _: &WpSinglePixelBufferManagerV1,
        _: wp_single_pixel_buffer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        unreachable!("wp_single_pixel_buffer_manager_v1::Event is empty in version 1")
    }
}
impl Dispatch<WlBuffer, ()> for DimData {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_buffer::Event::Release => debug!("WlBuffer released"),
            _ => todo!(),
        }
    }
}

impl Drop for DimView {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.buffer.destroy();
    }
}
