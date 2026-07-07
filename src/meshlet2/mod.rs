use crate::vkutils;

mod build_meshlets;
pub mod gltf;
mod gpu;
pub mod pipeline;
pub mod push_constants;

// TODO cleanup all these struct duplicates. Some are probably only local during asset creation
pub struct GeometryData {
    pub vertices: vkutils::buffer::Buffer,
    pub meshlet_vertices: vkutils::buffer::Buffer,
    pub meshlet_triangles: vkutils::buffer::Buffer,
    pub meshlets: vkutils::buffer::Buffer,
    pub mesh_instances: vkutils::buffer::Buffer,
    pub mesh_instances_count: u32,
    pub meshlet_instances: vkutils::buffer::Buffer,
    pub meshlet_instances_count: u32,
}
