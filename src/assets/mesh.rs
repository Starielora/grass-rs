use super::primitive::Primitive;
use crate::vkutils::{self, vk_destroy::VkDestroy};
use ash::vk;

pub struct Mesh {
    pub _name: Option<std::string::String>,
    pub primitives: std::vec::Vec<Primitive>,
    pub per_parent_node_transform:
        std::collections::HashMap<usize, (vkutils::buffer::Buffer, vk::DeviceAddress)>,
}

impl Mesh {
    pub fn create_model_buffer(&mut self, parent: usize, ctx: &vkutils::context::VulkanContext) {
        if self.per_parent_node_transform.contains_key(&parent) {
            todo!("Huhge");
        }

        let buffer = ctx.create_bar_buffer(
            std::mem::size_of::<glm::Mat4>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let address = buffer.device_address.unwrap();

        self.per_parent_node_transform
            .insert(parent, (buffer, address));
    }

    pub fn set_transformation(&self, parent: usize, transformation: &glm::Mat4) {
        let (buffer, _address) = self
            .per_parent_node_transform
            .get(&parent)
            .expect("Parent not set");

        buffer.update_contents(&[*transformation]);
    }

    pub fn get_transformation(&self, parent: usize) -> vk::DeviceAddress {
        let (_buffer, address) = self
            .per_parent_node_transform
            .get(&parent)
            .expect("Parent not set");

        *address
    }
}

impl std::ops::Drop for Mesh {
    fn drop(&mut self) {
        for (_, (buffer, _address)) in &self.per_parent_node_transform {
            buffer.vk_destroy();
        }
    }
}
