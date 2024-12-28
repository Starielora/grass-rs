use crate::gltf_loader;
use crate::vkutils;
use ash::vk;

pub struct MeshData {
    device: ash::Device,
    pub vertex_buffer: vk::Buffer, // TODO this would be better to be mod private. I need to move skybox
    vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    pub indices_count: usize,
}

fn upload_buffer<T: std::marker::Copy>(
    vertex_data: &Vec<T>,
    buffer_usage: vk::BufferUsageFlags,
    ctx: &vkutils::Context,
) -> (vk::Buffer, vk::DeviceMemory) {
    let (staging_buffer, staging_buffer_memory, staging_buffer_allocation_size) = ctx
        .create_buffer(
            (vertex_data.len() * std::mem::size_of::<T>()) as u64,
            buffer_usage | vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

    let (device_buffer, device_buffer_memory, _device_buffer_allocation_size) = ctx.create_buffer(
        (vertex_data.len() * std::mem::size_of::<T>()) as u64,
        buffer_usage | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    unsafe {
        let buffer_ptr = ctx
            .device
            .map_memory(
                staging_buffer_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Could not map cube buffer memory");

        ash::util::Align::new(
            buffer_ptr,
            std::mem::align_of::<T>() as u64,
            staging_buffer_allocation_size,
        )
        .copy_from_slice(&vertex_data.as_slice());

        ctx.device.unmap_memory(staging_buffer_memory);
    };

    let command_buffer = ctx.create_command_buffer(vk::CommandBufferLevel::PRIMARY, true);
    let region = [vk::BufferCopy::default().size(staging_buffer_allocation_size)];
    unsafe {
        ctx.device
            .cmd_copy_buffer(command_buffer, staging_buffer, device_buffer, &region);
    }
    ctx.flush_command_buffer(command_buffer, true);

    unsafe {
        ctx.device.free_memory(staging_buffer_memory, None);
        ctx.device.destroy_buffer(staging_buffer, None);
    }

    (device_buffer, device_buffer_memory)
}

impl MeshData {
    pub fn new(gltf_file_path: &str, ctx: &vkutils::Context) -> Self {
        let (vertex_data, index_data) = gltf_loader::load(gltf_file_path);

        let (vertex_buffer, vertex_buffer_memory) =
            upload_buffer(&vertex_data, vk::BufferUsageFlags::VERTEX_BUFFER, &ctx);
        let (index_buffer, index_buffer_memory) =
            upload_buffer(&index_data, vk::BufferUsageFlags::INDEX_BUFFER, &ctx);

        Self {
            device: ctx.device.clone(),
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            indices_count: index_data.len(),
        }
    }
}

impl std::ops::Drop for MeshData {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.destroy_buffer(self.index_buffer, None);
        }
    }
}
