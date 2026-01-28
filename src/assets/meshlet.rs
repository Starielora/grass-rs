use ash::vk;
use meshopt::ffi::meshopt_Meshlet;

use crate::vkutils::{self, push_constants::GPUPushConstants, vk_destroy::VkDestroy};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GPUMeshlet {
    pub vertices: [u32; 64],
    pub indices: [u32; 126 * 3],
    pub triangle_count: u32,
    pub vertex_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MeshletBounds {
    center: glm::Vec3,
    radius: f32,
    cone_apex: glm::Vec3,
    cone_cutoff: f32,
    cone_axis: glm::Vec3,
    // TODO its actually signed int - I wanted to avoid another glsl extensions
    remaining_cone_data: f32,
}

pub struct Meshlet {
    pub meshlet_buffer: vkutils::buffer::Buffer,
    pub vertex_buffer: vkutils::buffer::Buffer,
    pub meshlet_vertices: vkutils::buffer::Buffer,
    pub triangle_buffer: vkutils::buffer::Buffer,
    pub meshlet_bounds_buffer: vkutils::buffer::Buffer,
    pub meshlets_count: u32,
    pub bounds_count: u32,
}

impl Meshlet {
    pub fn cmd_draw(
        &self,
        device: &ash::Device,
        mesh_shader_device: &ash::ext::mesh_shader::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        push_constants.meshlet_data = self.meshlet_buffer.device_address.unwrap();
        push_constants.mesh_vertex_data = self.vertex_buffer.device_address.unwrap();
        push_constants.mesh_triangle_data = self.triangle_buffer.device_address.unwrap();
        push_constants.meshlet_vertex_indices = self.meshlet_vertices.device_address.unwrap();
        push_constants.meshlet_bounds_data = self.meshlet_bounds_buffer.device_address.unwrap();
        push_constants.meshlets_count = self.bounds_count; // TODO !!!!!

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

            mesh_shader_device.cmd_draw_mesh_tasks(command_buffer, self.meshlets_count / 64, 1, 1);
        }
    }
}

impl std::ops::Drop for Meshlet {
    fn drop(&mut self) {
        self.meshlet_buffer.vk_destroy();
        self.vertex_buffer.vk_destroy();
        self.meshlet_vertices.vk_destroy();
        self.triangle_buffer.vk_destroy();
        self.meshlet_bounds_buffer.vk_destroy();
    }
}

pub fn build_meshlets2(
    vertices: &std::vec::Vec<f32>,
    indices: &std::vec::Vec<u32>,
) -> (meshopt::Meshlets, std::vec::Vec<MeshletBounds>) {
    let vertices_slice = unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * std::mem::size_of::<f32>(),
        )
    };
    // TODO this kurwa stride is giga bad, consider using strongly typed vector
    let vertex_adapter =
        meshopt::VertexDataAdapter::new(vertices_slice, std::mem::size_of::<f32>() * 8, 0)
            .expect("Failed to create vertex adapter");

    // TODO revise max vertices and triangle count - fix in shaders as well
    // TODO use cone weight, when implementing cone culling
    let mut meshopt_meshlets =
        meshopt::build_meshlets(indices.as_slice(), &vertex_adapter, 64, 124, 0.0);

    // TODO does it really work?
    for meshlet in meshopt_meshlets.meshlets.iter_mut() {
        unsafe {
            meshopt::ffi::meshopt_optimizeMeshlet(
                meshopt_meshlets
                    .vertices
                    .as_mut_ptr()
                    .offset(meshlet.vertex_offset as isize),
                meshopt_meshlets
                    .triangles
                    .as_mut_ptr()
                    .offset(meshlet.triangle_offset as isize),
                meshlet.triangle_count as usize,
                meshlet.vertex_count as usize,
            )
        };
    }

    let mut meshlets_bounds = vec![];

    // TODO repack the data, to reduce size. Cone data perhaps unnecessary
    for meshlet in meshopt_meshlets.iter() {
        let meshlet_bounds = meshopt::compute_meshlet_bounds(meshlet, &vertex_adapter);
        // Repack the data, cuz sth is wrong with padding somewhere
        meshlets_bounds.push(MeshletBounds {
            center: glm::make_vec3(&meshlet_bounds.center),
            radius: meshlet_bounds.radius,
            cone_apex: glm::make_vec3(&meshlet_bounds.cone_apex),
            cone_cutoff: meshlet_bounds.cone_cutoff,
            cone_axis: glm::make_vec3(&meshlet_bounds.cone_axis),
            remaining_cone_data: 0.0f32,
        });
    }

    // TODO perhaps fix
    // draw divides meshlets count by 64, so last meshlets are getting cut from draw
    while meshopt_meshlets.meshlets.len() % 64 != 0 {
        meshopt_meshlets.meshlets.push(meshopt_Meshlet {
            vertex_offset: 0,
            triangle_offset: 0,
            vertex_count: 0,
            triangle_count: 0,
        });
    }

    (meshopt_meshlets, meshlets_bounds)
}
