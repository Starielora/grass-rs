use crate::gltf_loader;
use crate::vkutils;
use crate::vkutils_new;
use crate::vkutils_new::vk_destroy::VkDestroy;
use ash::vk;

pub struct MeshData {
    pub vertex_buffer: vkutils_new::buffer::Buffer, // TODO this would be better to be mod private. I need to move skybox
    pub index_buffer: vkutils_new::buffer::Buffer,
    pub indices_count: usize,
}

fn upload_buffer<T: std::marker::Copy>(
    vertex_data: &Vec<T>,
    buffer_usage: vk::BufferUsageFlags,
    ctx: &mut vkutils::Context,
) -> vkutils_new::buffer::Buffer {
    let mut staging_buffer = ctx.create_buffer(
        vertex_data.len() * std::mem::size_of::<T>(),
        buffer_usage | vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    let device_buffer = ctx.create_buffer(
        vertex_data.len() * std::mem::size_of::<T>(),
        buffer_usage | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    staging_buffer.update_contents(&vertex_data.as_slice());
    staging_buffer.unmap_memory();

    ctx.transient_transfer_command_pool
        .execute_short_lived_command_buffer(ctx.transfer_queue, |device, command_buffer| {
            let region = [vk::BufferCopy::default().size(staging_buffer.allocation_size)];
            unsafe {
                device.cmd_copy_buffer(
                    command_buffer,
                    staging_buffer.handle,
                    device_buffer.handle,
                    &region,
                );
            }
        });

    staging_buffer.vk_destroy();

    device_buffer
}

impl MeshData {
    pub fn new(gltf_file_path: &str, ctx: &mut vkutils::Context) -> Self {
        let (vertex_data, index_data) = gltf_loader::load(gltf_file_path);

        let vertex_buffer = upload_buffer(&vertex_data, vk::BufferUsageFlags::VERTEX_BUFFER, ctx);
        let index_buffer = upload_buffer(&index_data, vk::BufferUsageFlags::INDEX_BUFFER, ctx);

        Self {
            vertex_buffer,
            index_buffer,
            indices_count: index_data.len(),
        }
    }
}

impl std::ops::Drop for MeshData {
    fn drop(&mut self) {
        self.vertex_buffer.vk_destroy();
        self.index_buffer.vk_destroy();
    }
}
