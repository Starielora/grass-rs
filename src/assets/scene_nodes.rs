use super::gltf_asset::{Node, Scene};
use super::mesh::Mesh;
use crate::vkutils;
use ash::vk;

pub(super) struct SceneNodesBuffers {
    pub device: ash::Device,
    pub memory: vk::DeviceMemory,
    pub buffers: Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
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

pub(super) fn build_node_transformation_data(
    ctx: &vkutils::context::VulkanContext,
    meshes: &mut Vec<Mesh>,
    nodes: &Vec<Node>,
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

fn count_scene_instances(scene: &Scene, nodes: &Vec<Node>) -> usize {
    let mut instances_count: usize = 0;
    for node_index in &scene.nodes {
        instances_count += count_instances(*node_index, &nodes);
    }
    instances_count
}

fn create_buffers(
    ctx: &vkutils::context::VulkanContext,
    scene: &Scene,
    nodes: &Vec<Node>,
    single_instance_size: usize,
) -> (
    vk::DeviceMemory,
    Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
) {
    let instances_count = count_scene_instances(scene, nodes);
    let total_size = single_instance_size * instances_count;

    let mut model_data_buffers: Vec<vk::Buffer> = vec![];

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

    let mut data_buffers_with_addr: Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)> =
        vec![];

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

fn count_instances(node_index: usize, nodes: &Vec<Node>) -> usize {
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
    nodes: &Vec<Node>,
    meshes: &mut Vec<Mesh>,
    buffers: &mut Vec<(vk::Buffer, vk::DeviceAddress, *mut std::ffi::c_void)>,
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
