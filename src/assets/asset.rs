use super::mesh::Mesh;
use super::node::Node;
use super::scene::Scene;

use crate::vkutils::push_constants::GPUPushConstants;
use ash::vk;

pub struct Asset {
    pub meshes: std::vec::Vec<Mesh>,
    pub nodes: std::vec::Vec<Node>, // TODO cleanup, don't depend on gltf_loader
    pub scenes: std::vec::Vec<Scene>,
    pub default_scene: Option<usize>,
}

impl Asset {
    pub fn draw_scene(
        &self,
        index: usize,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        let scene = &self.scenes[index];
        for node in &scene.nodes {
            cmd_draw(
                *node,
                &self,
                device,
                command_buffer,
                pipeline_layout,
                push_constants,
            )
        }
    }
}

fn cmd_draw(
    node_index: usize,
    asset: &Asset,
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    pipeline_layout: vk::PipelineLayout,
    push_constants: &mut GPUPushConstants,
) {
    let node = &asset.nodes[node_index];
    if let Some(mesh_index) = node.mesh_index {
        let mesh = asset.meshes.iter().nth(mesh_index).unwrap();

        push_constants.mesh_data = mesh.get_transformation(node_index);

        for primitive in &mesh.primitives {
            primitive.cmd_draw(device, command_buffer, pipeline_layout, push_constants);
        }
    }

    for child in &node.children {
        cmd_draw(
            *child,
            asset,
            device,
            command_buffer,
            pipeline_layout,
            push_constants,
        );
    }
}
