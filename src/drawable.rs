use ash::vk;

pub trait Drawable {
    fn cmd_draw(&mut self, command_buffer: &vk::CommandBuffer);
}
