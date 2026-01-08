use crate::gltf_loader;
use crate::vkutils;
use crate::vkutils::vk_destroy::VkDestroy;
use ash::vk;

pub struct MeshData {
    pub vertex_buffer: vkutils::buffer::Buffer, // TODO this would be better to be mod private. I need to move skybox
    pub index_buffer: vkutils::buffer::Buffer,
    pub indices_count: usize,
    pub index_type: ash::vk::IndexType,
}

impl MeshData {
    pub fn new(gltf_file_path: &str, ctx: &vkutils::context::VulkanContext) -> Self {
        println!("Loading gltf: {}", gltf_file_path);
        let (vertex_data, index_data) = gltf_loader::load(gltf_file_path);

        let vertex_buffer = ctx.upload_buffer(&vertex_data, vk::BufferUsageFlags::VERTEX_BUFFER);
        let (index_buffer, indices_count, index_type) = match index_data {
            gltf_loader::IndexBufferType::U16(items) => (
                ctx.upload_buffer(&items, vk::BufferUsageFlags::INDEX_BUFFER),
                items.len(),
                ash::vk::IndexType::UINT16,
            ),
            gltf_loader::IndexBufferType::U32(items) => (
                ctx.upload_buffer(&items, vk::BufferUsageFlags::INDEX_BUFFER),
                items.len(),
                ash::vk::IndexType::UINT32,
            ),
        };

        Self {
            vertex_buffer,
            index_buffer,
            indices_count,
            index_type,
        }
    }
}

impl std::ops::Drop for MeshData {
    fn drop(&mut self) {
        self.vertex_buffer.vk_destroy();
        self.index_buffer.vk_destroy();
    }
}
