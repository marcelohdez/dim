use std::{collections::HashMap, time::Instant};

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
                wl_pointer, wl_touch,
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

use crate::{
    buffer::{BufferManager, BufferType},
    consts::INIT_SIZE,
    DimOpts, DimSurface,
};

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
    start_time: Instant,
    fade_sec: f32,
    fade_done: bool,
    surfaces: HashMap<WlOutput, DimSurface>,

    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    touch: Option<wl_touch::WlTouch>,
    exit: bool,
}

impl DimData {
    /// Generate a new instance of our app
    pub fn new(
        compositor: CompositorState,
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        layer_shell: LayerShell,
        opts: DimOpts,
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

            alpha: opts.alpha(),
            passthrough: opts.passthrough,
            start_time: Instant::now(),
            fade_sec: opts.fade(),
            fade_done: false,
            surfaces: HashMap::new(),

            keyboard: None,
            pointer: None,
            touch: None,
            exit: false,
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
        back_buffer: BufferType,
        output: &WlOutput,
    ) -> DimSurface {
        let layer = self.layer_shell.create_layer_surface(
            qh,
            self.compositor.create_surface(qh),
            Layer::Overlay,
            Some("dim_layer"),
            Some(output),
        );

        let (width, height) = if let Some((width, height)) = self
            .output_state
            .info(output)
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

        DimSurface::new(qh, buffer, back_buffer, viewport, layer)
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
        let Some(view) = self
            .surfaces
            .values_mut()
            .find(|view| view.layer() == layer)
        else {
            error!("Configuring layer not in list?");
            return;
        };

        let (width, height) = configure.new_size;
        view.set_size(width, height);

        view.draw(qh, !self.fade_done);
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
        surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        let Some((_, view)) = self
            .surfaces
            .iter_mut()
            .find(|(_, view)| view.layer().wl_surface() == surface)
        else {
            panic!("Frame event received for surface we do not own.")
        };

        let elapsed_sec = self.start_time.elapsed().as_millis() as f32 / 1000.;

        if !self.fade_done {
            let alpha = (self.alpha * (elapsed_sec / self.fade_sec)).clamp(0., self.alpha);
            match &mut self.buffer_mgr {
                BufferManager::SinglePixel(..) => {
                    view.set_back_buffer(self.buffer_mgr.get_buffer(qh, alpha));
                }
                BufferManager::Shm(_, pool) => {
                    if let BufferType::Shared(buffer) = view.back_buffer_mut() {
                        let canvas = buffer.canvas(pool).expect("Canvas is not drawable.");
                        BufferManager::paint(canvas, alpha);
                    }
                }
            }

            if elapsed_sec > self.fade_sec {
                self.fade_done = true;
                debug!("Fade done!")
            }
        }

        view.draw(qh, !self.fade_done);
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
        let buffer = self.buffer_mgr.get_buffer(qh, 0.);
        let back_buffer = self.buffer_mgr.get_buffer(qh, 0.);
        let view = self.new_surface(qh, buffer, back_buffer, &output);
        self.surfaces.insert(output, view);
    }

    fn update_output(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &QueueHandle<Self>,
        output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) {
        let buffer = self.buffer_mgr.get_buffer(qh, 0.);
        let back_buffer = self.buffer_mgr.get_buffer(qh, 0.);
        let new_view = self.new_surface(qh, buffer, back_buffer, &output);

        if let Some(view) = self.surfaces.get_mut(&output) {
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
        self.surfaces.remove(&output);
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
