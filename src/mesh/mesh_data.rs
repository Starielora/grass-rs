use crate::gltf_loader;
use crate::vkutils_new;
use crate::vkutils_new::vk_destroy::VkDestroy;
use ash::vk;

pub struct MeshData {
    pub vertex_buffer: vkutils_new::buffer::Buffer, // TODO this would be better to be mod private. I need to move skybox
    pub index_buffer: vkutils_new::buffer::Buffer,
    pub indices_count: usize,
}

impl MeshData {
    pub fn new(gltf_file_path: &str, ctx: &vkutils_new::context::VulkanContext) -> Self {
        let (vertex_data, index_data) = gltf_loader::load(gltf_file_path);

        let vertex_buffer = ctx.upload_buffer(&vertex_data, vk::BufferUsageFlags::VERTEX_BUFFER);
        let index_buffer = ctx.upload_buffer(&index_data, vk::BufferUsageFlags::INDEX_BUFFER);

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
