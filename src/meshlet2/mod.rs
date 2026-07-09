use ash::vk;

use crate::{meshlet2::gpu::GeometryBuildData, vkutils};

mod build_meshlets;
pub mod gltf;
mod gpu;
pub mod pipeline;

// TODO cleanup all these struct duplicates. Some are probably only local during asset creation
pub struct GeometryBuffers {
    pub vertices: vkutils::buffer::Buffer,
    pub meshlet_vertices: vkutils::buffer::Buffer,
    pub meshlet_triangles: vkutils::buffer::Buffer,
    pub meshes: vkutils::buffer::Buffer,
    pub meshlets: vkutils::buffer::Buffer,
    pub mesh_instances: vkutils::buffer::Buffer,
    pub mesh_instances_count: u32,
    pub meshlet_instances: vkutils::buffer::Buffer,
    pub meshlet_instances_count: u32,
}

impl GeometryBuffers {
    pub fn new(ctx: &vkutils::context::VulkanContext, data: &GeometryBuildData) -> Self {
        let meshlets_buffer = ctx.upload_buffer(
            &data.meshlets,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );
        let meshlets_vertices_buffer = ctx.upload_buffer(
            &data.meshlet_vertices,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );
        let meshlets_triangles_buffer = ctx.upload_buffer(
            &data.meshlet_triangles,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );
        let vertex_buffer = ctx.upload_buffer(
            &data.vertices,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );
        let mesh_instances_buffer = ctx.upload_buffer(
            &data.mesh_instances,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let meshlet_instances_buffer = ctx.upload_buffer(
            &data.meshlet_instances,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let meshes_buffer = ctx.upload_buffer(
            &data.meshes,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        Self {
            vertices: vertex_buffer,
            meshlet_vertices: meshlets_vertices_buffer,
            meshlet_triangles: meshlets_triangles_buffer,
            meshes: meshes_buffer,
            meshlets: meshlets_buffer,
            mesh_instances: mesh_instances_buffer,
            mesh_instances_count: data.mesh_instances.len() as u32,
            meshlet_instances: meshlet_instances_buffer,
            meshlet_instances_count: data.meshlet_instances.len() as u32,
        }
    }
}

impl vkutils::vk_destroy::VkDestroy for GeometryBuffers {
    fn vk_destroy(&self) {
        self.vertices.vk_destroy();
        self.meshlet_vertices.vk_destroy();
        self.meshlet_triangles.vk_destroy();
        self.meshes.vk_destroy();
        self.meshlets.vk_destroy();
        self.mesh_instances.vk_destroy();
        self.meshlet_instances.vk_destroy();
    }
}

pub struct GraphicsPipeline {
    vk: ash::Device,
    vk_ext: ash::ext::mesh_shader::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    view_camera_bda: vk::DeviceAddress,
}

impl std::ops::Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            let vk = &self.vk;
            vk.destroy_pipeline_layout(self.pipeline_layout, None);
            vk.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl GraphicsPipeline {
    pub fn new(
        vk: &ash::Device,
        vk_ext: &ash::ext::mesh_shader::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
        swapchain_format: vk::Format,
        depth_format: vk::Format,
        view_camera: vk::DeviceAddress,
    ) -> Self {
        let (pipeline, pipeline_layout) =
            pipeline::create_pipeline(vk, descriptor_set_layout, swapchain_format, depth_format);

        Self {
            vk: vk.clone(),
            vk_ext: vk_ext.clone(),
            pipeline,
            pipeline_layout,
            view_camera_bda: view_camera,
        }
    }

    pub fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
        geometry_data: &GeometryBuffers,
    ) {
        let vk = &self.vk;
        let vk_ext = &self.vk_ext;

        unsafe {
            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            let viewport = vk::Viewport {
                width: extent.width as f32,
                height: extent.height as f32,
                max_depth: 1.0,
                ..Default::default()
            };
            let scissors = vk::Rect2D {
                extent: extent,
                ..Default::default()
            };
            vk.cmd_set_viewport(command_buffer, 0, &[viewport]);
            vk.cmd_set_scissor(command_buffer, 0, &[scissors]);

            let pc = gpu::PushConstants {
                view_camera: self.view_camera_bda,
                vertices: geometry_data.vertices.device_address.unwrap(),
                meshlet_vertices: geometry_data.meshlet_vertices.device_address.unwrap(),
                meshlet_triangles: geometry_data.meshlet_triangles.device_address.unwrap(),
                meshes: geometry_data.meshes.device_address.unwrap(),
                meshlets: geometry_data.meshlets.device_address.unwrap(),
                mesh_instances: geometry_data.mesh_instances.device_address.unwrap(),
                meshlet_instances: geometry_data.meshlet_instances.device_address.unwrap(),
                mesh_instances_count: geometry_data.mesh_instances_count,
                meshlet_instances_count: geometry_data.meshlet_instances_count,
            };

            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::MESH_EXT
                    | vk::ShaderStageFlags::TASK_EXT
                    | vk::ShaderStageFlags::FRAGMENT,
                0,
                pc.data(),
            );

            // One task workgroup per 64 meshlet instances (== subgroup width, so the
            // task shader's single-subgroup ballot compaction stays correct).
            vk_ext.cmd_draw_mesh_tasks(
                command_buffer,
                (geometry_data.meshlet_instances_count + 63) / 64,
                1,
                1,
            );
        }
    }
}
