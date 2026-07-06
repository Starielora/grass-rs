use ash::vk;
use glm;

use crate::{assets::gltf_asset, meshlet2::gltf::GlobalGeometryData, vkutils};

mod build_meshlets;
mod gltf;
mod gpu;
pub mod pipeline;
pub mod push_constants;

// TODO cleanup all these struct duplicates. Some are probably only local during asset creation
pub struct GeometryDataHandles {
    pub vertices: vkutils::buffer::Buffer,
    pub meshlet_vertices: vkutils::buffer::Buffer,
    pub meshlet_triangles: vkutils::buffer::Buffer,
    pub meshlets: vkutils::buffer::Buffer,
}

pub struct Asset {
    pub scene_geometry_instances_transforms: vkutils::buffer::Buffer,
    pub scene_meshlet_instances: vkutils::buffer::Buffer,
    pub scene_meshlet_instances_count: u32,
}

impl Asset {
    // TODO split preparing geometry from uploading to GPU?
    // Probably yes, because it will allow me to load many assets into single buffers
    pub fn from_gltf(
        ctx: &vkutils::context::VulkanContext,
        gltf_asset: &gltf_asset::GltfAssetData,
    ) -> (GeometryDataHandles, std::vec::Vec<Self>) {
        let mut global_geometry_data = GlobalGeometryData {
            vertices: vec![],
            meshlet_vertices: vec![],
            meshlet_triangles: vec![],
            meshlets: vec![],
            meshlets_info: vec![],
        };

        let (per_scene_geometry_instances_transforms, per_scene_meshlet_instances) =
            gltf::parse(&gltf_asset, &mut global_geometry_data);

        let buffers_upload_time = std::time::Instant::now();
        {
            let meshlets_buffer = ctx.upload_buffer(
                &global_geometry_data.meshlets,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let meshlets_vertices_buffer = ctx.upload_buffer(
                &global_geometry_data.meshlet_vertices,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let meshlets_triangles_buffer = ctx.upload_buffer(
                &global_geometry_data.meshlet_triangles,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let vertex_buffer = ctx.upload_buffer(
                &global_geometry_data.vertices,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let mut out: std::vec::Vec<Self> = vec![];
            for (i, geometry_instances) in
                per_scene_geometry_instances_transforms.iter().enumerate()
            {
                let meshlet_instances = &per_scene_meshlet_instances[i];
                let geometry_instances_buf = ctx.upload_buffer(
                    geometry_instances,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );

                let meshlet_instances_buf = ctx.upload_buffer(
                    meshlet_instances,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );

                out.push(Self {
                    scene_geometry_instances_transforms: geometry_instances_buf,
                    scene_meshlet_instances: meshlet_instances_buf,
                    scene_meshlet_instances_count: meshlet_instances.len() as u32,
                });
            }

            println!("Buffers upload time: {:?}", buffers_upload_time.elapsed());

            let handles = GeometryDataHandles {
                vertices: vertex_buffer,
                meshlet_vertices: meshlets_vertices_buffer,
                meshlet_triangles: meshlets_triangles_buffer,
                meshlets: meshlets_buffer,
            };

            return (handles, out);
        }
    }
}
