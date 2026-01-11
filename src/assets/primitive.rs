use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
use crate::vkutils::vk_destroy::VkDestroy;
use ash::vk;

pub struct Primitive {
    pub vertex_buffer: vkutils::buffer::Buffer, // TODO this would be better to be mod private. I need to move skybox
    pub index_buffer: vkutils::buffer::Buffer,
    pub indices_count: usize,
    pub index_type: ash::vk::IndexType,
}

impl Primitive {
    pub fn cmd_draw(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        unsafe {
            device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            let vertex_buffers = [self.vertex_buffer.handle];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.handle,
                0,
                self.index_type,
            );
            device.cmd_draw_indexed(command_buffer, self.indices_count as u32, 1, 0, 0, 0);
        }
    }
}

impl std::ops::Drop for Primitive {
    fn drop(&mut self) {
        self.vertex_buffer.vk_destroy();
        self.index_buffer.vk_destroy();
    }
}
