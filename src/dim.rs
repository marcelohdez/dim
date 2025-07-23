use log::{debug, error, warn};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm, delegate_simple, delegate_touch,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            globals::GlobalList,
            protocol::{
                wl_buffer::{self, WlBuffer},
                wl_keyboard,
                wl_output::WlOutput,
                wl_pointer, wl_shm, wl_touch,
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
        touch::TouchHandler,
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface},
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};

use crate::{consts::INIT_SIZE, surface::BufferType, DimSurface};

pub struct DimData {
    compositor: CompositorState,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    layer_shell: LayerShell,
    buffer_mgr: BufferManager,
    viewporter: SimpleGlobal<WpViewporter, 1>,

    alpha: f32,
    passthrough: bool,
    surfaces: Vec<DimSurface>,

    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    touch: Option<wl_touch::WlTouch>,
    exit: bool,
}

/// Abstracts away which is the best buffer manager available
enum BufferManager {
    SinglePixel(SimpleGlobal<WpSinglePixelBufferManagerV1, 1>),
    /// Used as fallback, when single pixel buffer is not available
    Shm(Shm, SlotPool),
}

impl BufferManager {
    /// Generate a new buffer from the owned buffer manager type
    fn get_buffer(&mut self, qh: &QueueHandle<DimData>, alpha: f32) -> BufferType {
        match self {
            BufferManager::SinglePixel(simple_global) => {
                // pre-multiply alpha
                let alpha = (u32::MAX as f32 * alpha) as u32;

                BufferType::Wl(
                    simple_global
                        .get()
                        .expect("failed to get buffer")
                        .create_u32_rgba_buffer(0, 0, 0, alpha, qh, ()),
                )
            }

            // create a singe pixel buffer ourselves (to be resized by viewporter as well)
            BufferManager::Shm(_, pool) => {
                let (buffer, canvas) = pool
                    .create_buffer(1, 1, 4, wl_shm::Format::Argb8888)
                    .expect("Failed to get buffer from slot pool!");

                // ARGB is actually backwards being little-endian, so we set BGR to 0 for black so
                (0..3).for_each(|i| {
                    canvas[i] = 0;
                });
                // then, we set pre-multiplied alpha
                canvas[3] = (u8::MAX as f32 * alpha) as u8;

                BufferType::Shared(buffer)
            }
        }
    }
}

impl DimData {
    /// Generate a new instance of our app
    pub fn new(
        compositor: CompositorState,
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        layer_shell: LayerShell,
        alpha: f32,
        passthrough: bool,
    ) -> Self {
        let buffer_mgr = match SimpleGlobal::<WpSinglePixelBufferManagerV1, 1>::bind(globals, qh) {
            Ok(sg) => BufferManager::SinglePixel(sg),
            Err(_) => {
                warn!("Single pixel buffer not available! Using fallback.");

                let shm = Shm::bind(globals, qh).expect("Could not create shm.");
                let pool = SlotPool::new(1, &shm).expect("Failed to create pool!");
                BufferManager::Shm(shm, pool)
            }
        };

        Self {
            compositor,
            registry_state: RegistryState::new(globals),
            seat_state: SeatState::new(globals, qh),
            output_state: OutputState::new(globals, qh),
            layer_shell,
            buffer_mgr,
            viewporter: SimpleGlobal::<wp_viewporter::WpViewporter, 1>::bind(globals, qh)
                .expect("wp_viewporter not available"),

            alpha,
            passthrough,
            surfaces: Vec::new(),

            exit: false,
            keyboard: None,
            pointer: None,
            touch: None,
        }
    }

    pub fn should_exit(&self) -> bool {
        self.exit
    }

    /// Create a new dimmed surface to show on the given output
    fn new_surface(
        &self,
        qh: &QueueHandle<Self>,
        buffer: BufferType,
        output: WlOutput,
    ) -> DimSurface {
        let layer = self.layer_shell.create_layer_surface(
            qh,
            self.compositor.create_surface(qh),
            Layer::Overlay,
            Some("dim_layer"),
            Some(&output),
        );

        let (width, height) = if let Some((width, height)) = self
            .output_state
            .info(&output)
            .and_then(|info| info.logical_size)
        {
            (width as u32, height as u32)
        } else {
            (INIT_SIZE, INIT_SIZE)
        };

        if self.passthrough {
            let input_region = Region::new(&self.compositor).expect("Failed to get a wl_region");
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_input_region(Some(input_region.wl_region()));
        } else {
            layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        }

        layer.set_exclusive_zone(-1);
        layer.set_size(width, height);
        layer.commit();

        let viewport = self
            .viewporter
            .get()
            .expect("wp_viewporter failed")
            .get_viewport(layer.wl_surface(), qh, ());

        DimSurface::new(qh, buffer, viewport, layer, output)
    }
}

impl LayerShellHandler for DimData {
    fn closed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
    ) {
        // should not be possible other than by the compositor (e.g. when disconnecting outputs),
        // which is okay as any input will make us exit
    }

    fn configure(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let Some(view) = self.surfaces.iter_mut().find(|view| view.layer() == layer) else {
            error!("Configuring layer not in self.views?");
            return;
        };

        let (width, height) = configure.new_size;
        view.set_size(width, height);
        view.viewport().set_destination(width as _, height as _);

        if view.first_configure() {
            view.draw(qh);
            view.set_first_configure(false);
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
        _qh: &QueueHandle<Self>,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        debug!("Frame");
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
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
        let buffer = self.buffer_mgr.get_buffer(qh, self.alpha);
        let view = self.new_surface(qh, buffer, output);
        self.surfaces.push(view);
    }

    fn update_output(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        let buffer = self.buffer_mgr.get_buffer(qh, self.alpha);
        let new_view = self.new_surface(qh, buffer, output);

        if let Some(view) = self
            .surfaces
            .iter_mut()
            .find(|v| v.output() == new_view.output())
        {
            *view = new_view;
        } else {
            error!("Updating output not in views list??");
        }
    }

    fn output_destroyed(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        self.surfaces.retain(|v| v.output() != &output);
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
            Capability::Touch => {
                self.touch = Some(
                    self.seat_state
                        .get_touch(qh, &seat)
                        .expect("Failed to get touch device!"),
                )
            }
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
            Capability::Touch => self
                .touch
                .take()
                .expect("Failed to remove touch device!")
                .release(),
            _ => debug!("Unknown capability removed: {capability}"),
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
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
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
        _layout: u32,
    ) {
        debug!("Modifiers updated");
    }
}
impl PointerHandler for DimData {
    fn pointer_frame(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for e in events {
            match e.kind {
                PointerEventKind::Enter { serial } => {
                    if self.alpha == 1.0 {
                        pointer.set_cursor(serial, None, 0, 0);
                    }
                }
                PointerEventKind::Leave { .. } => {}
                _ => {
                    debug!("Mouse event");
                    self.exit = true;
                }
            }
        }
    }
}
impl TouchHandler for DimData {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        _surface: smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _id: i32,
        _position: (f64, f64),
    ) {
        self.exit = true;
    }

    fn up(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        _id: i32,
    ) {
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _time: u32,
        _id: i32,
        _position: (f64, f64),
    ) {
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }

    fn cancel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _touch: &wl_touch::WlTouch) {}
}
impl ShmHandler for DimData {
    fn shm_state(&mut self) -> &mut Shm {
        match &mut self.buffer_mgr {
            BufferManager::Shm(shm, _) => shm,
            _ => unreachable!("Attempted to call shm_state() when not using shm."),
        }
    }
}

delegate_compositor!(DimData);
delegate_touch!(DimData);
delegate_layer!(DimData);
delegate_registry!(DimData);
delegate_pointer!(DimData);
delegate_keyboard!(DimData);
delegate_output!(DimData);
delegate_seat!(DimData);
delegate_simple!(DimData, WpViewporter, 1);
delegate_shm!(DimData);

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
            _ => unreachable!("WlBuffer only has Release event"),
        }
    }
}
