use super::mesh::Mesh;
use super::node::Node;
use super::scene::Scene;

use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
use ash::vk;

pub struct Asset {
    pub meshes: std::vec::Vec<Mesh>,
    pub nodes: std::vec::Vec<Node>, // TODO cleanup, don't depend on gltf_loader
    pub scenes: std::vec::Vec<Scene>,
    pub default_scene: Option<usize>,

    model_data_memory: vk::DeviceMemory,
    model_data_buffers: std::vec::Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,

    device: ash::Device, // to delete buffers
}

impl Asset {
    pub fn new(
        ctx: &vkutils::context::VulkanContext,
        mut meshes: std::vec::Vec<Mesh>,
        nodes: std::vec::Vec<Node>,
        scenes: std::vec::Vec<Scene>,
        default_scene: Option<usize>,
    ) -> Self {
        let mut instances_count: usize = 0;
        for scene in &scenes {
            for node_index in &scene.nodes {
                instances_count += count_instances(*node_index, &nodes, &mut meshes);
            }
        }

        println!("Instances count: {}", instances_count);

        let single_instance_size = std::mem::size_of::<glm::Mat4>();
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

        let model_data_memory =
            ctx.allocate_memory(memory_requirements, memory_property_flags, true);

        let memory_ptr = unsafe {
            ctx.device
                .map_memory(
                    model_data_memory,
                    0,
                    total_size as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to map memory.")
                .clone()
        };

        // bind buffers
        let mut model_data_buffers_with_addr: std::vec::Vec<(
            vk::Buffer,
            vk::DeviceAddress,
            *mut std::ffi::c_void,
        )> = vec![];
        let mut offset = 0;
        for buffer in &mut model_data_buffers {
            unsafe {
                ctx.device
                    .bind_buffer_memory(*buffer, model_data_memory, offset)
                    .expect("Failed to bind model data buffer to memory");
            }
            let buffer_address_info = vk::BufferDeviceAddressInfo {
                buffer: *buffer,
                ..Default::default()
            };

            let address = unsafe { ctx.device.get_buffer_device_address(&buffer_address_info) };
            let ptr = unsafe { memory_ptr.offset(offset.try_into().unwrap()) };
            model_data_buffers_with_addr.push((buffer.clone(), address, ptr));

            offset += single_instance_size as u64;
        }

        for scene in &scenes {
            for node_index in &scene.nodes {
                upload_model_data(
                    *node_index,
                    &glm::Mat4::identity(),
                    &nodes,
                    &mut meshes,
                    &mut model_data_buffers_with_addr,
                    &mut 0,
                );
            }
        }

        Self {
            meshes,
            nodes,
            scenes,
            default_scene,
            model_data_memory,
            model_data_buffers: model_data_buffers_with_addr,
            device: ctx.device.clone(),
        }
    }

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

impl std::ops::Drop for Asset {
    fn drop(&mut self) {
        for (buffer, _address, _ptr) in &self.model_data_buffers {
            // TODO use somehow vk_destroy trait?
            unsafe {
                self.device.destroy_buffer(*buffer, None);
            }
        }

        unsafe { self.device.free_memory(self.model_data_memory, None) };
    }
}

fn count_instances(
    node_index: usize,
    nodes: &std::vec::Vec<Node>,
    meshes: &mut std::vec::Vec<Mesh>,
) -> usize {
    let node = &nodes[node_index];

    let mut this_node_count = 0;

    if let Some(_mesh_index) = node.mesh_index {
        this_node_count += 1;
    }

    for child in &node.children {
        this_node_count += count_instances(*child, nodes, meshes);
    }

    this_node_count
}

fn upload_model_data(
    node_index: usize,
    parent_transform: &glm::Mat4,
    nodes: &std::vec::Vec<Node>,
    meshes: &mut std::vec::Vec<Mesh>,
    buffers: &mut std::vec::Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
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
        mesh.set_model_buffer(node_index, *address);
    }

    for child in &node.children {
        upload_model_data(
            *child,
            &transform,
            nodes,
            meshes,
            buffers,
            current_buffer_index,
        );
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
