use ash::vk;

use crate::{
    meshlet2::gpu::{GeometryBuildData, MeshletInstance},
    vkutils::{self},
};

pub mod bounding_sphere;
mod build_meshlets;
pub mod compute;
pub mod gltf;
pub mod gpu;
pub mod pipeline;

pub struct GeometryBuffers {
    pub vertices: vkutils::buffer::Buffer,
    pub meshlet_vertices: vkutils::buffer::Buffer,
    pub meshlet_triangles: vkutils::buffer::Buffer,
    pub meshes: vkutils::buffer::Buffer,
    pub meshlets: vkutils::buffer::Buffer,
    pub mesh_instances: vkutils::buffer::Buffer,
    pub mesh_instances_count: u32,
    pub meshlet_instances: vkutils::buffer::Buffer,
    pub _meshlet_instances_count: u32,
    pub visible_meshlets_instances: vkutils::buffer::Buffer,
    pub visible_meshlets_instances_count: vkutils::buffer::Buffer,
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

        // stub - is overwritten in compute prepass
        let mut visible_meshlets_instances: std::vec::Vec<gpu::MeshletInstance> = vec![];
        visible_meshlets_instances.resize(
            data.meshlet_instances.len(),
            MeshletInstance {
                mesh_instance_index: 0,
                meshlet_index: 0,
                lod_index: 0,
            },
        );
        let visible_meshlets_instances_buffer = ctx.upload_buffer(
            &visible_meshlets_instances,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let visible_meshlets_instances_count_buffer = ctx.create_bar_buffer(
            std::mem::size_of::<u32>(),
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
            _meshlet_instances_count: data.meshlet_instances.len() as u32,
            visible_meshlets_instances: visible_meshlets_instances_buffer,
            visible_meshlets_instances_count: visible_meshlets_instances_count_buffer,
        }
    }

    pub fn push_constants(
        &self,
        view_camera: vk::DeviceAddress,
        meshlet_instances_draws: vk::DeviceAddress,
        draws_count_buffer: vk::DeviceAddress,
    ) -> gpu::PushConstants {
        gpu::PushConstants {
            view_camera,
            vertices: self.vertices.device_address.unwrap(),
            meshlet_vertices: self.meshlet_vertices.device_address.unwrap(),
            meshlet_triangles: self.meshlet_triangles.device_address.unwrap(),
            meshes: self.meshes.device_address.unwrap(),
            meshlets: self.meshlets.device_address.unwrap(),
            mesh_instances: self.mesh_instances.device_address.unwrap(),
            meshlet_instances: self.meshlet_instances.device_address.unwrap(),
            visible_meshlet_instances: meshlet_instances_draws,
            visible_meshlet_instances_count: draws_count_buffer,
            mesh_instances_count: self.mesh_instances_count,
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
        self.visible_meshlets_instances.vk_destroy();
        self.visible_meshlets_instances_count.vk_destroy();
    }
}

pub struct GraphicsPipeline {
    vk: ash::Device,
    vk_ext: ash::ext::mesh_shader::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    view_camera_bda: vk::DeviceAddress,
    draw_mesh_tasks_command_buf: vk::Buffer,
    pub lod: u32,
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
        draw_mesh_tasks_command_buf: vk::Buffer,
        subgroup_size: u32,
    ) -> Self {
        let (pipeline, pipeline_layout) = pipeline::create_pipeline(
            vk,
            descriptor_set_layout,
            swapchain_format,
            depth_format,
            subgroup_size,
        );

        Self {
            vk: vk.clone(),
            vk_ext: vk_ext.clone(),
            pipeline,
            pipeline_layout,
            view_camera_bda: view_camera,
            draw_mesh_tasks_command_buf,
            lod: 0,
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

            let pc = geometry_data.push_constants(
                self.view_camera_bda,
                geometry_data
                    .visible_meshlets_instances
                    .device_address
                    .unwrap(),
                geometry_data
                    .visible_meshlets_instances_count
                    .device_address
                    .unwrap(),
            );

            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                gpu::get_push_constants_stage_flags(),
                0,
                pc.data(),
            );

            vk_ext.cmd_draw_mesh_tasks_indirect(
                command_buffer,
                self.draw_mesh_tasks_command_buf,
                0,
                1,
                std::mem::size_of::<vk::DrawMeshTasksIndirectCommandEXT>() as u32,
            );
        }
    }
}
