use super::mesh::Mesh;
use super::node::Node;
use super::scene::Scene;

use crate::assets::DrawType;
use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
use crate::vkutils::vk_destroy::VkDestroy;
use ash::vk;

struct SceneNodesBuffers {
    pub device: ash::Device,
    pub memory: vk::DeviceMemory,
    pub buffers: std::vec::Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
    pub node_transform_buffer_address: std::collections::HashMap<usize, vk::DeviceAddress>,
}

impl std::ops::Drop for SceneNodesBuffers {
    fn drop(&mut self) {
        unsafe {
            for (buffer, _address, _ptr) in &self.buffers {
                self.device.destroy_buffer(*buffer, None);
            }

            self.device.free_memory(self.memory, None);
        }
    }
}

pub struct Asset {
    pub meshes: std::vec::Vec<Mesh>,
    pub nodes: std::vec::Vec<Node>, // TODO cleanup, don't depend on gltf_loader
    pub scenes: std::vec::Vec<Scene>,
    pub default_scene: Option<usize>,

    // TODO structure this properly. Take into consideration that scene indices may not correspond to vector indices
    mesh_type: DrawType,
    per_scene_node_transformation_data: std::vec::Vec<SceneNodesBuffers>,
    per_scene_node_meshlet_data: std::vec::Vec<vkutils::buffer::Buffer>,
    per_scene_draw_mesh_tasks_indirect_buffers: std::vec::Vec<(vkutils::buffer::Buffer, usize)>,

    fvf_instances_buffers: std::vec::Vec<vkutils::buffer::Buffer>,
    fvf_offsets_buffers: std::vec::Vec<vkutils::buffer::Buffer>,
    fvf_indirect_draw_buffers: Vec<(vkutils::buffer::Buffer, usize)>,

}

impl Asset {
    pub fn new(
        ctx: &vkutils::context::VulkanContext,
        mut meshes: std::vec::Vec<Mesh>,
        nodes: std::vec::Vec<Node>,
        scenes: std::vec::Vec<Scene>,
        default_scene: Option<usize>,
        mesh_type: DrawType,
    ) -> Self {
        let mut per_scene_node_transformation_data = vec![];
        let mut per_scene_node_meshlet_data = vec![];
        let mut per_scene_draw_mesh_tasks_indirect_buffers = vec![];
        let mut fvf_instances_buffers = vec![];
        let mut fvf_offsets_buffers = vec![];
        let mut fvf_indirect_draw_buffers = vec![];

        for scene in &scenes {
            let node_transformation_data =
                build_node_transformation_data(ctx, &mut meshes, &nodes, &scene);

            if let DrawType::FixedVertexFunctionCombined = mesh_type {
                let (offsets_buffer, instances_buffer) = fvf_build_instance_data(
                    ctx,
                    &node_transformation_data.node_transform_buffer_address,
                    &meshes,
                );
                let indirect_draw_buffers = fvf_build_indirect_buffer(ctx, &meshes);
                fvf_offsets_buffers.push(offsets_buffer);
                fvf_instances_buffers.push(instances_buffer);
                fvf_indirect_draw_buffers.push(indirect_draw_buffers);
            }

            if let DrawType::Meshlet = mesh_type {
                let instances_buffer = build_instance_data(
                    ctx,
                    &node_transformation_data.node_transform_buffer_address,
                    &meshes,
                    &nodes,
                );

                let draw_mesh_tasks_buffer = build_buffer_for_indirect_draw(ctx, &meshes, &nodes);

                per_scene_node_meshlet_data.push(instances_buffer);
                per_scene_draw_mesh_tasks_indirect_buffers.push(draw_mesh_tasks_buffer);
            }
            per_scene_node_transformation_data.push(node_transformation_data);
        }

        Self {
            meshes,
            nodes,
            scenes,
            default_scene,
            mesh_type,
            per_scene_node_transformation_data,
            per_scene_node_meshlet_data,
            per_scene_draw_mesh_tasks_indirect_buffers,
            fvf_instances_buffers,
            fvf_offsets_buffers,
            fvf_indirect_draw_buffers,
        }
    }

    pub fn draw_scene(
        &self,
        index: usize,
        device: &ash::Device,
        mesh_shader_device: &ash::ext::mesh_shader::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        let scene = &self.scenes[index];

        match self.mesh_type {
            DrawType::FixedVertexFunctionCombined => {
                for mesh in &self.meshes {
                    if let super::mesh::Primitives::FixedVertexFunctionCombined(primitives) = &mesh.primitives {
                        push_constants.fvf_instances =
                            self.fvf_instances_buffers[index].device_address.unwrap();
                        push_constants.fvf_instance_offsets =
                            self.fvf_offsets_buffers[index].device_address.unwrap();

                        let (indirect_buf, draw_count) = &self.fvf_indirect_draw_buffers[index];

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
            DrawType::FixedFunctionVertex => {
                for (mi, mesh) in self.meshes.iter().enumerate() {
                    let mut mesh_nodes = vec![];
                    for ni in &scene.nodes {
                        build_mesh_nodes(&mut mesh_nodes, &self, *ni, mi);
                    }

                    // TODO replace with draw_indexed
                    for node_index in &mesh_nodes {
                        push_constants.mesh_data = *self.per_scene_node_transformation_data[index]
                            .node_transform_buffer_address
                            .get(node_index)
                            .unwrap() as u64;

                        if let super::mesh::Primitives::FixedFunctionVertexPrimitives(primitives) =
                            &mesh.primitives
                        {
                            for primitive in primitives {
                                primitive.cmd_draw(
                                    device,
                                    command_buffer,
                                    pipeline_layout,
                                    push_constants,
                                );
                            }
                        }
                    }
                }
            }
            DrawType::Meshlet => {
                push_constants.meshlet_draw = self.per_scene_node_meshlet_data[index]
                    .device_address
                    .unwrap();
                let (buf, draws_count) = &self.per_scene_draw_mesh_tasks_indirect_buffers[index];
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
    }
}

impl std::ops::Drop for Asset {
    fn drop(&mut self) {
        for buf in &self.per_scene_node_meshlet_data {
            buf.vk_destroy();
        }

        for (buf, _size) in &self.per_scene_draw_mesh_tasks_indirect_buffers {
            buf.vk_destroy();
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct MeshInstance_MeshletDraw {
    pub mesh_data: vk::DeviceAddress, // model matrix buffer address
    pub meshlet_data: vk::DeviceAddress,
    pub mesh_vertex_data: vk::DeviceAddress,
    pub meshlet_vertices: vk::DeviceAddress,
    pub meshlet_triangles: vk::DeviceAddress,
    pub meshlet_bounds: vk::DeviceAddress,
    pub meshlets_count: u32,
}

fn fvf_build_indirect_buffer(
    ctx: &vkutils::context::VulkanContext,
    meshes: &[Mesh],
) -> (vkutils::buffer::Buffer, usize) {
    let mut draws = vec![];
    for mesh in meshes {
        if let super::mesh::Primitives::FixedVertexFunctionCombined(primitives) = &mesh.primitives {
            for (i, &index_count) in primitives.primitive_index_count.iter().enumerate() {
                draws.push(vk::DrawIndexedIndirectCommand {
                    index_count,
                    instance_count: primitives.primitive_parent_node_indices[i].len() as u32,
                    first_index: primitives.primitive_index_offset_in_combined_index_buffer[i],
                    vertex_offset: primitives.primitive_vertex_offset_in_combined_vertex_buffer[i] as i32,
                    first_instance: 0,
                });
            }
        }
    }
    let draw_count = draws.len();
    let buffer = ctx.upload_buffer(&draws, vk::BufferUsageFlags::INDIRECT_BUFFER);
    (buffer, draw_count)
}

fn build_buffer_for_indirect_draw(
    ctx: &vkutils::context::VulkanContext,
    meshes: &std::vec::Vec<Mesh>,
    nodes: &std::vec::Vec<Node>,
) -> (vkutils::buffer::Buffer, usize) {
    let mut draws = vec![];
    // TODO same loop as in build_node_meshlet_data
    for node in nodes {
        if let Some(mesh_index) = node.mesh_index {
            let mesh = meshes.iter().nth(mesh_index).unwrap();
            match &mesh.primitives {
                super::mesh::Primitives::FixedVertexFunctionCombined(_) => {
                    todo!();
                }
                super::mesh::Primitives::FixedFunctionVertexPrimitives(_) => {
                    todo!(
                        "ale sie kurwa zjebalo - ten branch nie powinien byc wykonywany tu nigdy"
                    );
                }
                super::mesh::Primitives::Meshlets(meshlets) => {
                    for meshlet in meshlets {
                        let draw = vk::DrawMeshTasksIndirectCommandEXT {
                            group_count_x: meshlet.meshlets_count / 64,
                            group_count_y: 1,
                            group_count_z: 1,
                        };
                        draws.push(draw);
                    }
                }
            }
        }
    }

    let buffer = ctx.upload_buffer(&draws, vk::BufferUsageFlags::INDIRECT_BUFFER);

    (buffer, draws.len())
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FVFInstanceData {
    pub mesh_data_br: vk::DeviceAddress,
}

fn fvf_build_instance_data(
    ctx: &vkutils::context::VulkanContext,
    node_transform_buffer_address: &std::collections::HashMap<usize, vk::DeviceAddress>,
    meshes: &std::vec::Vec<Mesh>,
) -> (vkutils::buffer::Buffer, vkutils::buffer::Buffer) {
    let mut instance_data = vec![];
    let mut instance_offset = vec![]; // offset in instance buffer
    for mesh in meshes {
        if let super::mesh::Primitives::FixedVertexFunctionCombined(primitives) = &mesh.primitives {
            let mut offset = 0 as u32;
            for node_indices in &primitives.primitive_parent_node_indices {
                for node_index in node_indices {
                    instance_data.push(FVFInstanceData {
                        mesh_data_br: *node_transform_buffer_address.get(node_index).unwrap(),
                    });
                }
                instance_offset.push(offset);
                offset += node_indices.len() as u32;
            }
        }
    }

    let offsets_buf = ctx.upload_buffer(
        &instance_offset,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER
            | ash::vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );
    let instances_buf = ctx.upload_buffer(
        &instance_data,
        ash::vk::BufferUsageFlags::STORAGE_BUFFER
            | ash::vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    (offsets_buf, instances_buf)
}

fn build_instance_data(
    ctx: &vkutils::context::VulkanContext,
    node_transform_buffer_address: &std::collections::HashMap<usize, vk::DeviceAddress>,
    meshes: &std::vec::Vec<Mesh>,
    nodes: &std::vec::Vec<Node>,
) -> vkutils::buffer::Buffer {
    let mut meshlet_draws = vec![];

    for (node_index, node) in nodes.iter().enumerate() {
        if let Some(mesh_index) = node.mesh_index {
            let mesh = meshes.iter().nth(mesh_index).unwrap();
            match &mesh.primitives {
                super::mesh::Primitives::FixedFunctionVertexPrimitives(_) => {
                    todo!(
                        "ale sie kurwa zjebalo - ten branch nie powinien byc wykonywany tu nigdy"
                    );
                }
                super::mesh::Primitives::Meshlets(meshlets) => {
                    for meshlet in meshlets {
                        let draw = MeshInstance_MeshletDraw {
                            mesh_data: *node_transform_buffer_address.get(&node_index).unwrap()
                                as vk::DeviceAddress,
                            meshlet_data: meshlet.meshlet_buffer.device_address.unwrap(),
                            mesh_vertex_data: meshlet.vertex_buffer.device_address.unwrap(),
                            meshlet_vertices: meshlet.meshlet_vertices.device_address.unwrap(),
                            meshlet_triangles: meshlet.triangle_buffer.device_address.unwrap(),
                            meshlet_bounds: meshlet.meshlet_bounds_buffer.device_address.unwrap(),
                            meshlets_count: meshlet.bounds_count,
                        };
                        meshlet_draws.push(draw);
                    }
                }
                super::mesh::Primitives::FixedVertexFunctionCombined(_) => {
                    todo!();
                }
            }
        }
    }

    let buffer = ctx.upload_buffer(
        &meshlet_draws,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    buffer
}

fn count_scene_instances(scene: &Scene, nodes: &std::vec::Vec<Node>) -> usize {
    let mut instances_count: usize = 0;
    for node_index in &scene.nodes {
        instances_count += count_instances(*node_index, &nodes);
    }
    instances_count
}

fn create_buffers(
    ctx: &vkutils::context::VulkanContext,
    scene: &Scene,
    nodes: &std::vec::Vec<Node>,
    single_instance_size: usize,
) -> (
    vk::DeviceMemory,
    std::vec::Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
) {
    let instances_count = count_scene_instances(scene, nodes);
    let total_size = single_instance_size * instances_count;

    let mut model_data_buffers: std::vec::Vec<vk::Buffer> = vec![];

    let mut memory_requirements = Option::None;
    for _ in 0..instances_count {
        let (buffer, mem_reqs) = ctx.create_unbound_buffer(
            single_instance_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );
        model_data_buffers.push(buffer);
        memory_requirements = Some(mem_reqs);
    }

    let mut memory_requirements = memory_requirements.unwrap().clone();
    memory_requirements.size = total_size as u64;

    // bar buffer
    // TODO maybe I don't need it as bar buffer. Also maybe I don't even need to map it.
    // Initially was done to play around with primitives transformations, which DeviceAddress
    // makes very comfortable
    let memory_property_flags = vk::MemoryPropertyFlags::HOST_VISIBLE
        | vk::MemoryPropertyFlags::HOST_COHERENT
        | vk::MemoryPropertyFlags::DEVICE_LOCAL;

    let data_memory = ctx.allocate_memory(memory_requirements, memory_property_flags, true);

    let memory_ptr = unsafe {
        ctx.device
            .map_memory(
                data_memory,
                0,
                total_size as u64,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map memory.")
            .clone()
    };

    let mut data_buffers_with_addr: std::vec::Vec<(
        vk::Buffer,
        vk::DeviceAddress,
        *mut std::ffi::c_void,
    )> = vec![];

    let mut offset = 0;
    for buffer in &mut model_data_buffers {
        unsafe {
            ctx.device
                .bind_buffer_memory(*buffer, data_memory, offset)
                .expect("Failed to bind model data buffer to memory");
        }
        let buffer_address_info = vk::BufferDeviceAddressInfo {
            buffer: *buffer,
            ..Default::default()
        };

        let address = unsafe { ctx.device.get_buffer_device_address(&buffer_address_info) };
        let ptr = unsafe { memory_ptr.offset(offset.try_into().unwrap()) };
        data_buffers_with_addr.push((buffer.clone(), address, ptr));

        offset += single_instance_size as u64;
    }
    (data_memory, data_buffers_with_addr)
}

fn build_node_transformation_data(
    ctx: &vkutils::context::VulkanContext,
    meshes: &mut std::vec::Vec<Mesh>,
    nodes: &std::vec::Vec<Node>,
    scene: &Scene,
) -> SceneNodesBuffers {
    let (model_data_memory, mut model_data_buffers_with_addr) =
        create_buffers(ctx, scene, nodes, std::mem::size_of::<glm::Mat4>());
    let mut node_transform_buffer_address: std::collections::HashMap<usize, vk::DeviceAddress> =
        std::collections::HashMap::new();
    let mut current_buffer_index = 0;
    for node_index in &scene.nodes {
        upload_model_data(
            *node_index,
            &glm::Mat4::identity(),
            &nodes,
            meshes,
            &mut model_data_buffers_with_addr,
            &mut node_transform_buffer_address,
            &mut current_buffer_index,
        );
    }

    SceneNodesBuffers {
        device: ctx.device.clone(),
        memory: model_data_memory,
        buffers: model_data_buffers_with_addr,
        node_transform_buffer_address,
    }
}

fn count_instances(node_index: usize, nodes: &std::vec::Vec<Node>) -> usize {
    let node = &nodes[node_index];

    let mut this_node_count = 0;

    if let Some(_mesh_index) = node.mesh_index {
        this_node_count += 1;
    }

    for child in &node.children {
        this_node_count += count_instances(*child, nodes);
    }

    this_node_count
}

fn upload_model_data(
    node_index: usize,
    parent_transform: &glm::Mat4,
    nodes: &std::vec::Vec<Node>,
    meshes: &mut std::vec::Vec<Mesh>,
    buffers: &mut std::vec::Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
    node_transform_buffer_address: &mut std::collections::HashMap<usize, vk::DeviceAddress>,
    current_buffer_index: &mut usize,
) {
    let node = &nodes[node_index];
    let transform = parent_transform * &node.matrix;
    if let Some(_mesh_index) = node.mesh_index {
        let (_buffer, address, ptr) = buffers.iter().nth(*current_buffer_index).unwrap();
        *current_buffer_index += 1;
        let slice = [transform];
        unsafe {
            let mapped_slice = core::slice::from_raw_parts_mut(ptr.cast(), slice.len());
            mapped_slice.copy_from_slice(&slice);
        }
        if let Some(_) = node_transform_buffer_address.insert(node_index, *address) {
            panic!("Bro, there's a bug");
        }
    }

    for child in &node.children {
        upload_model_data(
            *child,
            &transform,
            nodes,
            meshes,
            buffers,
            node_transform_buffer_address,
            current_buffer_index,
        );
    }
}

fn build_mesh_nodes(mesh_nodes: &mut Vec<usize>, asset: &Asset, node_index: usize, mi: usize) {
    let node = &asset.nodes[node_index];
    if let Some(mesh_index) = node.mesh_index {
        if mesh_index == mi {
            mesh_nodes.push(node_index);
        }
    }

    for child in &node.children {
        build_mesh_nodes(mesh_nodes, asset, *child, mi);
    }
}
