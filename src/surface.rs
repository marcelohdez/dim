use smithay_client_toolkit::{
    reexports::{client::QueueHandle, protocols::wp::viewporter::client::wp_viewport::WpViewport},
    shell::{wlr_layer::LayerSurface, WaylandSurface},
};

use crate::{buffer::BufferType, consts::INIT_SIZE, DimData};

pub struct DimSurface {
    width: u32,
    height: u32,

    buffer: BufferType,
    back_buffer: BufferType,

    viewport: WpViewport,
    layer: LayerSurface,
}

impl DimSurface {
    pub fn new(
        _qh: &QueueHandle<DimData>,
        buffer: BufferType,
        back_buffer: BufferType,
        viewport: WpViewport,
        layer: LayerSurface,
    ) -> Self {
        Self {
            width: INIT_SIZE,
            height: INIT_SIZE,

            buffer,
            back_buffer,
            viewport,
            layer,
        }
    }

    pub fn draw(&mut self, qh: &QueueHandle<DimData>, request_next: bool) {
        let wl_buffer = match &self.back_buffer {
            BufferType::Wl(wl_buffer) => wl_buffer,
            BufferType::Shared(buffer) => buffer.wl_buffer(),
        };

        self.layer.wl_surface().attach(Some(wl_buffer), 0, 0);
        self.layer
            .wl_surface()
            .damage(0, 0, self.width as _, self.height as _);
        std::mem::swap(&mut self.buffer, &mut self.back_buffer);

        if request_next {
            self.layer
                .wl_surface()
                .frame(qh, self.layer.wl_surface().clone());
        }

        self.layer.commit();
    }

    pub fn layer(&self) -> &LayerSurface {
        &self.layer
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.viewport
            .set_destination(self.width as _, self.height as _);
    }

    pub fn set_back_buffer(&mut self, back_buffer: BufferType) {
        self.back_buffer = back_buffer;
    }

    pub fn back_buffer_mut(&mut self) -> &mut BufferType {
        &mut self.back_buffer
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
