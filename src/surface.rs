use log::debug;
use smithay_client_toolkit::{
    reexports::{
        client::{
            protocol::{wl_buffer::WlBuffer, wl_output::WlOutput},
            QueueHandle,
        },
        protocols::wp::viewporter::client::wp_viewport::WpViewport,
    },
    shell::{wlr_layer::LayerSurface, WaylandSurface},
    shm::slot,
};

use crate::{consts::INIT_SIZE, DimData};

pub struct DimSurface {
    first_configure: bool,
    width: u32,
    height: u32,
    buffer: BufferType,
    viewport: WpViewport,
    layer: LayerSurface,
    output: WlOutput,
}

pub enum BufferType {
    Wl(WlBuffer),
    Shared(slot::Buffer),
}

impl DimSurface {
    pub fn new(
        _qh: &QueueHandle<DimData>,
        buffer: BufferType,
        viewport: WpViewport,
        layer: LayerSurface,
        output: WlOutput,
    ) -> Self {
        Self {
            first_configure: true,
            width: INIT_SIZE,
            height: INIT_SIZE,
            buffer,
            viewport,
            layer,
            output,
        }
    }

    pub fn draw(&mut self, _qh: &QueueHandle<DimData>) {
        debug!("Requesting draw");
        if !self.first_configure {
            // we only need to draw once as it is a static color
            return;
        }

        let wl_buffer = match &self.buffer {
            BufferType::Wl(wl_buffer) => wl_buffer,
            BufferType::Shared(buffer) => buffer.wl_buffer(),
        };

        self.layer.wl_surface().attach(Some(wl_buffer), 0, 0);
        self.layer.commit();

        debug!("Drawn");
    }

    pub fn width(&self) -> u32 {
        self.height
    }

    pub fn height(&self) -> u32 {
        self.width
    }

    pub fn first_configure(&self) -> bool {
        self.first_configure
    }

    pub fn output(&self) -> &WlOutput {
        &self.output
    }

    pub fn layer(&self) -> &LayerSurface {
        &self.layer
    }

    pub fn set_first_configure(&mut self, value: bool) {
        self.first_configure = value;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn viewport(&mut self) -> &mut WpViewport {
        &mut self.viewport
    }
}

impl Drop for DimSurface {
    fn drop(&mut self) {
        self.viewport.destroy();
        if let BufferType::Wl(buffer) = &mut self.buffer {
            buffer.destroy();
        }
    }
}
