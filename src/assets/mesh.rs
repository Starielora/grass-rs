use super::meshlet::Meshlet;
use super::primitive::Primitive;
use ash::vk;

pub enum Primitives {
    FixedFunctionVertexPrimitives(std::vec::Vec<Primitive>),
    Meshlets(std::vec::Vec<Meshlet>),
}

pub struct Mesh {
    pub _name: Option<std::string::String>,
    pub primitives: Primitives,
    pub per_parent_node_model_buffer: std::collections::HashMap<usize, vk::DeviceAddress>,
}

impl Mesh {
    pub fn set_model_buffer_address(&mut self, parent: usize, address: vk::DeviceAddress) {
        let oldval = self.per_parent_node_model_buffer.insert(parent, address);
        if let Some(_) = oldval {
            todo!("Huhge");
        }
    }

    pub fn get_transformation(&self, parent: usize) -> vk::DeviceAddress {
        let address = self
            .per_parent_node_model_buffer
            .get(&parent)
            .expect("Parent not set");

        *address
    }
}
