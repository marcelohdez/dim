use log::debug;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_simple,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            globals::GlobalList,
            protocol::{
                wl_buffer::{self, WlBuffer},
                wl_keyboard, wl_pointer,
            },
            Connection, Dispatch, QueueHandle,
        },
        protocols::wp::{
            single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::{
                self, WpSinglePixelBufferManagerV1,
            },
            viewporter::client::{
                wp_viewport::{self, WpViewport},
                wp_viewporter::WpViewporter,
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
        wlr_layer::{LayerShellHandler, LayerSurface},
        WaylandSurface,
    },
};

use crate::INIT_SIZE;

pub struct DimData {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,

    exit: bool,
    first_configure: bool,
    width: u32,
    height: u32,
    buffer: WlBuffer,
    viewport: WpViewport,
    layer: LayerSurface,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
}

impl DimData {
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        viewport: WpViewport,
        layer: LayerSurface,
    ) -> Self {
        Self {
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, qh),
            output_state: OutputState::new(globals, qh),

            exit: false,
            first_configure: true,
            width: INIT_SIZE,
            height: INIT_SIZE,
            buffer: {
                SimpleGlobal::<WpSinglePixelBufferManagerV1, 1>::bind(globals, qh)
                    .expect("wp_single_pixel_buffer_manager_v1 not available!")
                    .get()
                    .expect("Failed to get buffer manager")
                    .create_u32_rgba_buffer(0, 0, 0, u32::MAX, qh, ())
            },
            viewport,
            layer,
            keyboard: None,
            keyboard_focus: true,
            pointer: None,
        }
    }

    pub fn should_exit(&self) -> bool {
        self.exit
    }

    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());
        self.layer.wl_surface().attach(Some(&self.buffer), 0, 0);
        self.layer.commit();

        debug!("Drawn");
        // TODO save and reuse buffer when the window size is unchanged.
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
        self.draw(qh);
    }
}

impl OutputHandler for DimData {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
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
        if surface == self.layer.wl_surface() {
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
        if surface == self.layer.wl_surface() {
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
                PointerEventKind::Enter { .. } => debug!("Mouse entered!"),
                _ => self.exit = true,
            }
        }
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
        _layer: &LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        (self.width, self.height) = configure.new_size;

        self.viewport
            .set_destination(self.width as _, self.height as _);

        // Initiare first draw
        if self.first_configure {
            self.first_configure = false;
            self.draw(qh);
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

impl Drop for DimData {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.buffer.destroy();
    }
}
