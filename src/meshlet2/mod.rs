use ash::vk;

use crate::{meshlet2::gpu::GeometryBuildData, vkutils};

pub mod bounding_sphere;
mod build_meshlets;
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
    pub meshlet_instances_count: u32,
    pub per_lod_draws: std::vec::Vec<(vkutils::buffer::Buffer, vkutils::buffer::Buffer, u32, u32)>, // TODO this is temporary to test LOD. Meshlet instances will be set in compute prepass with according LOD
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

        let per_lod_draws = create_meshlets_to_draw_buffer(ctx, data);

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
            per_lod_draws,
        }
    }

    pub fn push_constants(
        &self,
        task_dispatches: vk::DeviceAddress,
        view_camera: vk::DeviceAddress,
        lod: u32,
        meshlet_instances_draws: vk::DeviceAddress,
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
            task_dispatches,
            meshlet_instances_draws,
            mesh_instances_count: self.mesh_instances_count,
            meshlet_instances_count: self.meshlet_instances_count,
            draws_count: self.per_lod_draws[lod as usize].2,
        }
    }
}

fn create_meshlets_to_draw_buffer(
    ctx: &vkutils::context::VulkanContext,
    data: &GeometryBuildData,
) -> std::vec::Vec<(vkutils::buffer::Buffer, vkutils::buffer::Buffer, u32, u32)> {
    let mut per_lod_buf: std::vec::Vec<std::vec::Vec<u32>> = vec![];
    per_lod_buf.resize(gpu::MAX_LODS, vec![]);

    for (meshlet_instance_index, meshlet_instance) in data.meshlet_instances.iter().enumerate() {
        let buf = &mut per_lod_buf[meshlet_instance.lod_index as usize];
        buf.push(meshlet_instance_index as u32);
    }

    let mut out: std::vec::Vec<(vkutils::buffer::Buffer, vkutils::buffer::Buffer, u32, u32)> =
        vec![];
    for buf in &per_lod_buf {
        let (dx, dy) = gpu::task_dispatch_2d(
            buf.len() as u32,
            ctx.physical_device.subgroup_size,
            ctx.physical_device.max_task_workgroup_count[0],
        );
        let dispatch = vk::DrawMeshTasksIndirectCommandEXT {
            group_count_x: dx,
            group_count_y: dy,
            group_count_z: 1,
        };
        let dispatch_buf = ctx.upload_buffer(
            &vec![dispatch],
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::INDIRECT_BUFFER,
        );
        out.push((
            ctx.upload_buffer(
                &buf,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            ),
            dispatch_buf,
            buf.len() as u32,
            1,
        ));
    }
    out
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

        for (buf1, buf2, _, _) in &self.per_lod_draws {
            buf1.vk_destroy();
            buf2.vk_destroy();
        }
    }
}

pub struct GraphicsPipeline {
    vk: ash::Device,
    vk_ext: ash::ext::mesh_shader::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    view_camera_bda: vk::DeviceAddress,
    task_dispatches_bda: vk::DeviceAddress,
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
        draw_params_bda: vk::DeviceAddress,
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
            task_dispatches_bda: draw_params_bda,
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

            let (draw_buf, dispatch_buf, _draws_count, elements_in_draw_buf) =
                &geometry_data.per_lod_draws[self.lod as usize];

            let pc = geometry_data.push_constants(
                self.task_dispatches_bda,
                self.view_camera_bda,
                self.lod,
                draw_buf.device_address.unwrap(),
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
                dispatch_buf.handle,
                0,
                *elements_in_draw_buf,
                std::mem::size_of::<vk::DrawMeshTasksIndirectCommandEXT>() as u32,
            );
        }
    }
}
