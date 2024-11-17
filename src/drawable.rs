use crate::push_constants::GPUPushConstants;
use ash::vk;

pub trait Drawable {
    fn cmd_draw(&mut self, command_buffer: &vk::CommandBuffer, push_constants: &GPUPushConstants);
}
