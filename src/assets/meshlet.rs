use ash::vk;

use crate::vkutils::{self, push_constants::GPUPushConstants, vk_destroy::VkDestroy};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GPUMeshlet {
    pub vertices: [u32; 64],
    pub indices: [u32; 126 * 3],
    pub triangle_count: u32,
    pub vertex_count: u32,
}

pub struct Meshlet {
    pub meshlet_buffer: vkutils::buffer::Buffer,
    pub vertex_buffer: vkutils::buffer::Buffer,
    pub meshlets_count: u32,
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

        unsafe {
            device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::MESH_EXT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            mesh_shader_device.cmd_draw_mesh_tasks(command_buffer, self.meshlets_count, 1, 1);
        }
    }
}

impl std::ops::Drop for Meshlet {
    fn drop(&mut self) {
        self.meshlet_buffer.vk_destroy();
        self.vertex_buffer.vk_destroy();
    }
}

// TODO this is trivial impl, use meshopt
pub fn build_meshlets(
    vertices: &std::vec::Vec<f32>,
    indices: &std::vec::Vec<u32>,
) -> std::vec::Vec<GPUMeshlet> {
    let mut meshlets = std::vec::Vec::new();
    let mut meshlet_vertices = std::vec::Vec::<u32>::with_capacity(vertices.len());
    meshlet_vertices.resize(vertices.len(), 0xFF_u32);

    let mut meshlet = GPUMeshlet {
        vertices: [0; 64],
        indices: [0; 126 * 3],
        triangle_count: 0,
        vertex_count: 0,
    };

    for i in (0..indices.len()).step_by(3) {
        let a = indices[i + 0];
        let b = indices[i + 1];
        let c = indices[i + 2];

        if meshlet.vertex_count
            + (meshlet_vertices[a as usize] == 0xFF_u32) as u32
            + (meshlet_vertices[b as usize] == 0xFF_u32) as u32
            + (meshlet_vertices[c as usize] == 0xFF_u32) as u32
            > 64
            || meshlet.triangle_count >= 126
        {
            meshlets.push(meshlet.clone());
            for j in 0..meshlet.vertex_count {
                meshlet_vertices[meshlet.vertices[j as usize] as usize] = 0xFF_u32;
            }

            meshlet = GPUMeshlet {
                vertices: [0; 64],
                indices: [0; 126 * 3],
                triangle_count: 0,
                vertex_count: 0,
            };
        }

        if meshlet_vertices[a as usize] == 0xFF_u32 {
            meshlet_vertices[a as usize] = meshlet.vertex_count;
            meshlet.vertices[meshlet.vertex_count as usize] = a as u32;
            meshlet.vertex_count += 1;
        }
        if meshlet_vertices[b as usize] == 0xFF_u32 {
            meshlet_vertices[b as usize] = meshlet.vertex_count;
            meshlet.vertices[meshlet.vertex_count as usize] = b as u32;
            meshlet.vertex_count += 1;
        }
        if meshlet_vertices[c as usize] == 0xFF_u32 {
            meshlet_vertices[c as usize] = meshlet.vertex_count;
            meshlet.vertices[meshlet.vertex_count as usize] = c as u32;
            meshlet.vertex_count += 1;
        }

        meshlet.indices[meshlet.triangle_count as usize * 3 + 0] = meshlet_vertices[a as usize];
        meshlet.indices[meshlet.triangle_count as usize * 3 + 1] = meshlet_vertices[b as usize];
        meshlet.indices[meshlet.triangle_count as usize * 3 + 2] = meshlet_vertices[c as usize];
        meshlet.triangle_count += 1;
    }

    if meshlet.triangle_count > 0 {
        meshlets.push(meshlet);
    }

    meshlets
}
