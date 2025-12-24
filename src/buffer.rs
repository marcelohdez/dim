use smithay_client_toolkit::{
    reexports::{
        client::{
            protocol::{wl_buffer::WlBuffer, wl_shm},
            QueueHandle,
        },
        protocols::wp::single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1,
    },
    registry::SimpleGlobal,
    shm::{
        slot::{self, SlotPool},
        Shm,
    },
};

use crate::DimData;

/// Abstracts away which is the best buffer manager available
pub enum BufferManager {
    SinglePixel(SimpleGlobal<WpSinglePixelBufferManagerV1, 1>),
    /// Should be used as fallback, when single pixel buffer is not available
    Shm(Shm, SlotPool),
}

pub enum BufferType {
    Wl(WlBuffer),
    Shared(slot::Buffer),
}

impl BufferManager {
    /// Generate a new buffer from the owned buffer manager type
    pub fn get_buffer(&mut self, qh: &QueueHandle<DimData>, alpha: f32) -> BufferType {
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

                BufferManager::paint(canvas, alpha);
                BufferType::Shared(buffer)
            }
        }
    }

    pub fn paint(canvas: &mut [u8], alpha: f32) {
        // RGB
        (0..3).for_each(|i| {
            canvas[i] = 0;
        });
        // ...A
        canvas[3] = (u8::MAX as f32 * alpha) as u8;
    }
}
