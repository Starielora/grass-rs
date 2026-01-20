use ash::vk;

use crate::vkutils::push_constants::GPUPushConstantsTraditional;

pub trait OverlayDrawable {
    fn record(&self, command_buffer: vk::CommandBuffer, push_constants: &mut GPUPushConstantsTraditional);
    fn enabled(&self) -> bool {
        true
    }
}
