use super::mesh::Mesh;
use super::node::Node;
use super::scene::Scene;

use crate::assets::MeshType;
use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
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

    device: ash::Device,
    per_scene_node_transformation_data: std::vec::Vec<SceneNodesBuffers>,
    per_scene_node_meshlet_data: std::vec::Vec<vkutils::buffer::Buffer>,
}

impl Asset {
    pub fn new(
        ctx: &vkutils::context::VulkanContext,
        mut meshes: std::vec::Vec<Mesh>,
        nodes: std::vec::Vec<Node>,
        scenes: std::vec::Vec<Scene>,
        default_scene: Option<usize>,
        mesh_type: MeshType,
    ) -> Self {
        let mut per_scene_node_transformation_data = vec![];
        let mut per_scene_node_meshlet_data = vec![];

        for scene in &scenes {
            let node_transformation_data =
                build_node_transformation_data(ctx, &mut meshes, &nodes, &scene);

            if let MeshType::Meshlet = mesh_type {
                let buffer = build_node_meshlet_data(
                    ctx,
                    &node_transformation_data.node_transform_buffer_address,
                    &mut meshes,
                    &nodes,
                    &scene,
                );
                per_scene_node_meshlet_data.push(buffer);
            }
            per_scene_node_transformation_data.push(node_transformation_data);
        }

        Self {
            meshes,
            nodes,
            scenes,
            default_scene,
            device: ctx.device.clone(),
            per_scene_node_transformation_data,
            per_scene_node_meshlet_data,
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
        for (mi, mesh) in self.meshes.iter().enumerate() {
            let mut mesh_nodes = vec![];
            for ni in &scene.nodes {
                build_mesh_nodes(&mut mesh_nodes, &self, *ni, mi);
            }

            // TODO replace with draw_indexed
            for node_index in &mesh_nodes {
                // push_constants.mesh_data = mesh.get_transformation(*node_index);
                push_constants.mesh_data = *self.per_scene_node_transformation_data[index]
                    .node_transform_buffer_address
                    .get(node_index)
                    .unwrap() as u64;

                match &mesh.primitives {
                    super::mesh::Primitives::FixedFunctionVertexPrimitives(primitives) => {
                        for primitive in primitives {
                            primitive.cmd_draw(
                                device,
                                command_buffer,
                                pipeline_layout,
                                push_constants,
                            );
                        }
                    }
                    super::mesh::Primitives::Meshlets(meshlets) => {
                        push_constants.meshlet_draw = self.per_scene_node_meshlet_data[index]
                            .device_address
                            .unwrap();
                        for meshlet in meshlets {
                            meshlet.cmd_draw(
                                device,
                                mesh_shader_device,
                                command_buffer,
                                pipeline_layout,
                                push_constants,
                            );
                        }
                    }
                }
            }

            println!("{}: {:?}", mi, mesh_nodes);
        }
    }
}

impl std::ops::Drop for Asset {
    fn drop(&mut self) {
        unsafe {
            // for (buffer, _address, _ptr) in &self.model_data_buffers_with_addr {
            //     self.device.destroy_buffer(*buffer, None);
            // }

            // self.device.free_memory(self.model_buffers_memory, None);
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

fn build_node_meshlet_data(
    ctx: &vkutils::context::VulkanContext,
    node_transform_buffer_address: &std::collections::HashMap<usize, vk::DeviceAddress>,
    meshes: &mut std::vec::Vec<Mesh>,
    nodes: &std::vec::Vec<Node>,
    scene: &Scene,
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
    // scenes: &std::vec::Vec<Scene>,
    scene: &Scene,
) -> SceneNodesBuffers {
    // let mut per_scene_node_transformation_data = vec![];
    // for scene in scenes {
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

    // per_scene_node_transformation_data.push(SceneNodesBuffers {
    //     device: ctx.device.clone(),
    //     memory: model_data_memory,
    //     buffers: model_data_buffers_with_addr,
    //     node_transform_buffer_address,
    // }
    SceneNodesBuffers {
        device: ctx.device.clone(),
        memory: model_data_memory,
        buffers: model_data_buffers_with_addr,
        node_transform_buffer_address,
    }
    // }

    // per_scene_node_transformation_data
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
    if let Some(mesh_index) = node.mesh_index {
        let mesh = &mut meshes[mesh_index];
        let (_buffer, address, ptr) = buffers.iter().nth(*current_buffer_index).unwrap();
        *current_buffer_index += 1;
        let slice = [transform];
        unsafe {
            let mapped_slice = core::slice::from_raw_parts_mut(ptr.cast(), slice.len());
            mapped_slice.copy_from_slice(&slice);
        }
        // this allows to share model matrix between mesh pipeline and fixed vertex pipeline
        mesh.set_model_buffer_address(node_index, *address);
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

fn cmd_draw(
    node_index: usize,
    asset: &Asset,
    device: &ash::Device,
    mesh_shader_device: &ash::ext::mesh_shader::Device,
    command_buffer: vk::CommandBuffer,
    pipeline_layout: vk::PipelineLayout,
    push_constants: &mut GPUPushConstants,
) {
    let node = &asset.nodes[node_index];
    if let Some(mesh_index) = node.mesh_index {
        let mesh = asset.meshes.iter().nth(mesh_index).unwrap();

        push_constants.mesh_data = mesh.get_transformation(node_index);

        match &mesh.primitives {
            super::mesh::Primitives::FixedFunctionVertexPrimitives(primitives) => {
                for primitive in primitives {
                    primitive.cmd_draw(device, command_buffer, pipeline_layout, push_constants);
                }
            }
            super::mesh::Primitives::Meshlets(meshlets) => {
                for meshlet in meshlets {
                    meshlet.cmd_draw(
                        device,
                        mesh_shader_device,
                        command_buffer,
                        pipeline_layout,
                        push_constants,
                    );
                }
            }
        }
    }

    for child in &node.children {
        cmd_draw(
            *child,
            asset,
            device,
            mesh_shader_device,
            command_buffer,
            pipeline_layout,
            push_constants,
        );
    }
}
