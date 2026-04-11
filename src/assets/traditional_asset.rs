use super::gltf_asset::{GltfAssetData, IndexBufferType, Node, Scene};
use super::mesh::{Mesh, Primitives};
use super::primitive::FVFCombinedPrimitives;
use super::scene_nodes::{build_node_transformation_data, SceneNodesBuffers};
use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
use crate::vkutils::vk_destroy::VkDestroy;
use ash::vk;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct TraditionalInstance {
    pub transform: vk::DeviceAddress,
}

pub struct TraditionalAsset {
    pub meshes: Vec<Mesh>,
    pub default_scene: Option<usize>,
    _node_transform_data: Vec<SceneNodesBuffers>,
    instances_buffers: Vec<vkutils::buffer::Buffer>,
    offsets_buffers: Vec<vkutils::buffer::Buffer>,
    indirect_draw_buffers: Vec<(vkutils::buffer::Buffer, usize)>,
}

impl TraditionalAsset {
    pub fn from_gltf(ctx: &vkutils::context::VulkanContext, asset_data: &GltfAssetData) -> Self {
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut primitive_vertex_offset_in_combined_vertex_buffer = vec![];
        let mut primitive_index_count = vec![];
        let mut primitive_index_offset_in_combined_index_buffer = vec![];
        let mut primitive_parent_node_indices = vec![];

        let mut vertex_offset_in_combined_vb = 0 as u32;
        let mut index_offset_in_combined_ib = 0 as u32;

        for (mesh_index, mesh) in asset_data.meshes.iter().enumerate() {
            for primitive in &mesh.primitives {
                let mut parent_node_indices: Vec<usize> = vec![];
                for (node_index, node) in asset_data.nodes.iter().enumerate() {
                    if let Some(node_mesh_index) = node.mesh_index {
                        if node_mesh_index == mesh_index {
                            parent_node_indices.push(node_index);
                        }
                    }
                }

                primitive_parent_node_indices.push(parent_node_indices);

                vertices.append(&mut primitive.vertex_buffer.clone());
                let vertex_count = (primitive.vertex_buffer.len() / 8) as u32;
                primitive_vertex_offset_in_combined_vertex_buffer
                    .push(vertex_offset_in_combined_vb);
                vertex_offset_in_combined_vb += vertex_count;

                // TODO coalesce all indices into u32. A bit wasteful memory-wise, but is a
                // superset for all current use cases.
                let mut ib = match &primitive.index_buffer {
                    IndexBufferType::U16(items) => {
                        items.iter().map(|i| *i as u32).collect::<Vec<_>>()
                    }
                    IndexBufferType::U32(items) => items.clone(),
                };
                primitive_index_count.push(ib.len() as u32);
                primitive_index_offset_in_combined_index_buffer.push(index_offset_in_combined_ib);
                index_offset_in_combined_ib += ib.len() as u32;
                indices.append(&mut ib);
            }
        }

        let vb = ctx.upload_buffer(&vertices, vk::BufferUsageFlags::VERTEX_BUFFER);
        let ib = ctx.upload_buffer(&indices, vk::BufferUsageFlags::INDEX_BUFFER);

        let combined = FVFCombinedPrimitives {
            vb,
            ib,
            primitive_vertex_offset_in_combined_vertex_buffer,
            primitive_index_count,
            primitive_index_offset_in_combined_index_buffer,
            primitive_parent_node_indices,
        };

        let mut meshes = vec![Mesh {
            _name: Some("combined".to_string()),
            primitives: Primitives::FixedVertexFunctionCombined(combined),
        }];

        let nodes: Vec<Node> = asset_data.nodes.clone();
        let scenes: Vec<Scene> = asset_data.scenes.clone();

        println!("Meshes count: {} (Traditional)", meshes.len());

        let mut node_transform_data = vec![];
        let mut instances_buffers = vec![];
        let mut offsets_buffers = vec![];
        let mut indirect_draw_buffers = vec![];

        for scene in &scenes {
            let transform_data = build_node_transformation_data(ctx, &mut meshes, &nodes, &scene);

            let (offsets_buffer, instances_buffer) = fvf_build_instance_data(
                ctx,
                &transform_data.node_transform_buffer_address,
                &meshes,
            );
            let indirect_buf = fvf_build_indirect_buffer(ctx, &meshes);

            offsets_buffers.push(offsets_buffer);
            instances_buffers.push(instances_buffer);
            indirect_draw_buffers.push(indirect_buf);
            node_transform_data.push(transform_data);
        }

        Self {
            meshes,
            default_scene: None,
            _node_transform_data: node_transform_data,
            instances_buffers,
            offsets_buffers,
            indirect_draw_buffers,
        }
    }

    pub fn draw_scene(
        &self,
        scene_index: usize,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        for mesh in &self.meshes {
            if let Primitives::FixedVertexFunctionCombined(primitives) = &mesh.primitives {
                push_constants.instances =
                    self.instances_buffers[scene_index].device_address.unwrap();
                push_constants.instance_offsets =
                    self.offsets_buffers[scene_index].device_address.unwrap();

                let (indirect_buf, draw_count) = &self.indirect_draw_buffers[scene_index];

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

                    device.cmd_bind_index_buffer(
                        command_buffer,
                        primitives.ib.handle,
                        0,
                        vk::IndexType::UINT32,
                    );

                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[primitives.vb.handle],
                        &[0],
                    );

                    device.cmd_draw_indexed_indirect(
                        command_buffer,
                        indirect_buf.handle,
                        0,
                        *draw_count as u32,
                        std::mem::size_of::<vk::DrawIndexedIndirectCommand>() as u32,
                    );
                }
            }
        }
    }
}

impl std::ops::Drop for TraditionalAsset {
    fn drop(&mut self) {
        for buf in &self.instances_buffers {
            buf.vk_destroy();
        }
        for buf in &self.offsets_buffers {
            buf.vk_destroy();
        }
        for (buf, _) in &self.indirect_draw_buffers {
            buf.vk_destroy();
        }
    }
}

fn fvf_build_indirect_buffer(
    ctx: &vkutils::context::VulkanContext,
    meshes: &[Mesh],
) -> (vkutils::buffer::Buffer, usize) {
    let mut draws = vec![];
    for mesh in meshes {
        if let Primitives::FixedVertexFunctionCombined(primitives) = &mesh.primitives {
            for (i, &index_count) in primitives.primitive_index_count.iter().enumerate() {
                draws.push(vk::DrawIndexedIndirectCommand {
                    index_count,
                    instance_count: primitives.primitive_parent_node_indices[i].len() as u32,
                    first_index: primitives.primitive_index_offset_in_combined_index_buffer[i],
                    vertex_offset: primitives.primitive_vertex_offset_in_combined_vertex_buffer[i]
                        as i32,
                    first_instance: 0,
                });
            }
        }
    }
    let draw_count = draws.len();
    let buffer = ctx.upload_buffer(&draws, vk::BufferUsageFlags::INDIRECT_BUFFER);
    (buffer, draw_count)
}

fn fvf_build_instance_data(
    ctx: &vkutils::context::VulkanContext,
    node_transform_buffer_address: &std::collections::HashMap<usize, vk::DeviceAddress>,
    meshes: &Vec<Mesh>,
) -> (vkutils::buffer::Buffer, vkutils::buffer::Buffer) {
    let mut instance_data = vec![];
    let mut instance_offset = vec![];
    for mesh in meshes {
        if let Primitives::FixedVertexFunctionCombined(primitives) = &mesh.primitives {
            let mut offset = 0 as u32;
            for node_indices in &primitives.primitive_parent_node_indices {
                for node_index in node_indices {
                    instance_data.push(TraditionalInstance {
                        transform: *node_transform_buffer_address.get(node_index).unwrap(),
                    });
                }
                instance_offset.push(offset);
                offset += node_indices.len() as u32;
            }
        }
    }

    let offsets_buf = ctx.upload_buffer(
        &instance_offset,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );
    let instances_buf = ctx.upload_buffer(
        &instance_data,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    (offsets_buf, instances_buf)
}
