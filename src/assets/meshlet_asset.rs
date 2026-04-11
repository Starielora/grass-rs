use super::gltf_asset::{GltfAssetData, IndexBufferType, Node, Scene};
use super::mesh::{Mesh, Primitives};
use super::meshlet::{build_meshlets2, Meshlet};
use super::scene_nodes::{build_node_transformation_data, SceneNodesBuffers};
use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
use crate::vkutils::vk_destroy::VkDestroy;
use ash::vk;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct MeshletDraw {
    pub transform: vk::DeviceAddress, // TransformBuf  — model matrix
    pub meshlets: vk::DeviceAddress,  // MeshletBuf
    pub vertices: vk::DeviceAddress,  // VertexBuf
    pub vertex_indices: vk::DeviceAddress, // VertexIndexBuf
    pub tri_indices: vk::DeviceAddress, // TriangleIndexBuf
    pub bounds: vk::DeviceAddress,    // MeshletBoundsBuf
    pub meshlets_count: u32,
}

pub struct MeshletAsset {
    pub _meshes: Vec<Mesh>,
    pub default_scene: Option<usize>,
    _node_transform_data: Vec<SceneNodesBuffers>,
    instance_buffers: Vec<vkutils::buffer::Buffer>,
    indirect_buffers: Vec<(vkutils::buffer::Buffer, usize)>,
}

impl MeshletAsset {
    pub fn from_gltf(ctx: &vkutils::context::VulkanContext, asset_data: &GltfAssetData) -> Self {
        let mut meshes: Vec<Mesh> = vec![];

        for mesh in &asset_data.meshes {
            let mut primitives: Vec<Meshlet> = vec![];

            for primitive in &mesh.primitives {
                let vertex_data = &primitive.vertex_buffer;
                let index_data: Vec<u32> = match &primitive.index_buffer {
                    IndexBufferType::U16(items) => {
                        items.iter().map(|u16val| *u16val as u32).collect()
                    }
                    IndexBufferType::U32(items) => items.clone(),
                };

                let (meshlets, bounds) = build_meshlets2(&vertex_data, &index_data);

                let meshlet_buffer = ctx.upload_buffer(
                    &meshlets.meshlets,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );
                let vertex_buffer = ctx.upload_buffer(
                    &vertex_data,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );
                let meshlet_vertices = ctx.upload_buffer(
                    &meshlets.vertices,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );
                let triangle_buffer = ctx.upload_buffer(
                    &meshlets.triangles,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );
                let meshlet_bounds_buffer = ctx.upload_buffer(
                    &bounds,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );

                primitives.push(Meshlet {
                    meshlet_buffer,
                    vertex_buffer,
                    meshlet_vertices,
                    triangle_buffer,
                    meshlet_bounds_buffer,
                    meshlets_count: meshlets.len() as u32,
                    bounds_count: bounds.len() as u32,
                });
            }
            meshes.push(Mesh {
                _name: mesh._name.clone(),
                primitives: Primitives::Meshlets(primitives),
            });
        }

        let nodes: Vec<Node> = asset_data.nodes.clone();
        let scenes: Vec<Scene> = asset_data.scenes.clone();

        println!("Meshes count: {} (Meshlet)", meshes.len());

        let mut node_transform_data = vec![];
        let mut instance_buffers = vec![];
        let mut indirect_buffers = vec![];

        for scene in &scenes {
            let transform_data = build_node_transformation_data(ctx, &mut meshes, &nodes, &scene);

            let instances_buffer = build_instance_data(
                ctx,
                &transform_data.node_transform_buffer_address,
                &meshes,
                &nodes,
            );
            let draw_buffer = build_buffer_for_indirect_draw(ctx, &meshes, &nodes);

            instance_buffers.push(instances_buffer);
            indirect_buffers.push(draw_buffer);
            node_transform_data.push(transform_data);
        }

        Self {
            _meshes: meshes,
            default_scene: None,
            _node_transform_data: node_transform_data,
            instance_buffers,
            indirect_buffers,
        }
    }

    pub fn draw_scene(
        &self,
        scene_index: usize,
        device: &ash::Device,
        mesh_shader_device: &ash::ext::mesh_shader::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        push_constants.meshlet_draws = self.instance_buffers[scene_index].device_address.unwrap();
        let (buf, draws_count) = &self.indirect_buffers[scene_index];
        unsafe {
            device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::TASK_EXT
                    | vk::ShaderStageFlags::MESH_EXT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );
            mesh_shader_device.cmd_draw_mesh_tasks_indirect(
                command_buffer,
                buf.handle,
                0,
                *draws_count as u32,
                12,
            );
        }
    }
}

impl std::ops::Drop for MeshletAsset {
    fn drop(&mut self) {
        for buf in &self.instance_buffers {
            buf.vk_destroy();
        }
        for (buf, _) in &self.indirect_buffers {
            buf.vk_destroy();
        }
    }
}

fn build_instance_data(
    ctx: &vkutils::context::VulkanContext,
    node_transform_buffer_address: &std::collections::HashMap<usize, vk::DeviceAddress>,
    meshes: &Vec<Mesh>,
    nodes: &Vec<Node>,
) -> vkutils::buffer::Buffer {
    let mut meshlet_draws = vec![];

    for (node_index, node) in nodes.iter().enumerate() {
        if let Some(mesh_index) = node.mesh_index {
            let mesh = meshes.iter().nth(mesh_index).unwrap();
            if let Primitives::Meshlets(meshlets) = &mesh.primitives {
                for meshlet in meshlets {
                    let draw = MeshletDraw {
                        transform: *node_transform_buffer_address.get(&node_index).unwrap()
                            as vk::DeviceAddress,
                        meshlets: meshlet.meshlet_buffer.device_address.unwrap(),
                        vertices: meshlet.vertex_buffer.device_address.unwrap(),
                        vertex_indices: meshlet.meshlet_vertices.device_address.unwrap(),
                        tri_indices: meshlet.triangle_buffer.device_address.unwrap(),
                        bounds: meshlet.meshlet_bounds_buffer.device_address.unwrap(),
                        meshlets_count: meshlet.bounds_count,
                    };
                    meshlet_draws.push(draw);
                }
            }
        }
    }

    ctx.upload_buffer(
        &meshlet_draws,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    )
}

fn build_buffer_for_indirect_draw(
    ctx: &vkutils::context::VulkanContext,
    meshes: &Vec<Mesh>,
    nodes: &Vec<Node>,
) -> (vkutils::buffer::Buffer, usize) {
    let mut draws = vec![];
    for node in nodes {
        if let Some(mesh_index) = node.mesh_index {
            let mesh = meshes.iter().nth(mesh_index).unwrap();
            if let Primitives::Meshlets(meshlets) = &mesh.primitives {
                for meshlet in meshlets {
                    draws.push(vk::DrawMeshTasksIndirectCommandEXT {
                        group_count_x: meshlet.meshlets_count / 64,
                        group_count_y: 1,
                        group_count_z: 1,
                    });
                }
            }
        }
    }

    let buffer = ctx.upload_buffer(&draws, vk::BufferUsageFlags::INDIRECT_BUFFER);
    (buffer, draws.len())
}
