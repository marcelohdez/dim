use log::debug;
use smithay_client_toolkit::{
    reexports::{client::QueueHandle, protocols::wp::viewporter::client::wp_viewport::WpViewport},
    shell::{wlr_layer::LayerSurface, WaylandSurface},
};

use crate::{buffer::BufferType, consts::INIT_SIZE, DimData};

pub struct DimSurface {
    first_configure: bool,
    width: u32,
    height: u32,
    buffer: BufferType,
    viewport: WpViewport,
    layer: LayerSurface,
}

impl DimSurface {
    pub fn new(
        _qh: &QueueHandle<DimData>,
        buffer: BufferType,
        viewport: WpViewport,
        layer: LayerSurface,
    ) -> Self {
        Self {
            first_configure: true,
            width: INIT_SIZE,
            height: INIT_SIZE,
            buffer,
            viewport,
            layer,
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
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn first_configure(&self) -> bool {
        self.first_configure
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

    pub fn viewport_mut(&mut self) -> &mut WpViewport {
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
