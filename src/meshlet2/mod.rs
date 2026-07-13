use ash::vk;

use crate::{
    meshlet2::gpu::{GeometryBuildData, MeshletInstance},
    vkutils::{
        self,
        shaders::{self, ShaderData},
    },
};

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
    pub draws_buffer: vkutils::buffer::Buffer,
    pub draws_count_buffer: vkutils::buffer::Buffer,
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

        // stub - is overwritten in compute prepass
        let mut draws_data: std::vec::Vec<gpu::MeshletInstance> = vec![];
        draws_data.resize(
            data.meshlet_instances.len(),
            MeshletInstance {
                mesh_instance_index: 0,
                meshlet_index: 0,
                lod_index: 0,
            },
        );
        let draws_buffer = ctx.upload_buffer(
            &draws_data,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let draws_count_buffer = ctx.create_bar_buffer(
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
            meshlet_instances_count: data.meshlet_instances.len() as u32,
            per_lod_draws,
            draws_buffer,
            draws_count_buffer,
        }
    }

    pub fn push_constants(
        &self,
        task_dispatches: vk::DeviceAddress,
        view_camera: vk::DeviceAddress,
        lod: u32,
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
            task_dispatches,
            meshlet_instances_draws,
            meshlet_instances_draws_count: draws_count_buffer,
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
    let mut per_lod_buf: std::vec::Vec<std::vec::Vec<MeshletInstance>> = vec![];
    per_lod_buf.resize(gpu::MAX_LODS, vec![]);

    for (meshlet_instance_index, meshlet_instance) in data.meshlet_instances.iter().enumerate() {
        let buf = &mut per_lod_buf[meshlet_instance.lod_index as usize];
        buf.push(meshlet_instance.clone());
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
    task_dispatches_handle: vk::Buffer,
    task_dispatches_bda: vk::DeviceAddress,
    compute_prepass_pipeline: vk::Pipeline,
    compute_prepass_pipeline_layout: vk::PipelineLayout,
    compute_prepass_dispatch_pipeline: vk::Pipeline,
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
        draw_params_handle: vk::Buffer,
        draw_params_bda: vk::DeviceAddress,
        subgroup_size: u32,
    ) -> Self {
        let (
            compute_prepass_pipeline,
            compute_prepass_pipeline_layout,
            compute_prepass_dispatch_pipeline,
        ) = create_compute_prepass_pipeline(vk, descriptor_set_layout, subgroup_size);

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
            task_dispatches_handle: draw_params_handle,
            task_dispatches_bda: draw_params_bda,
            lod: 0,
            compute_prepass_pipeline,
            compute_prepass_pipeline_layout,
            compute_prepass_dispatch_pipeline,
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

        geometry_data
            .draws_count_buffer
            .update_contents(&[0 as u32]);

        // prepass
        unsafe {
            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_prepass_pipeline,
            );

            let pc = geometry_data.push_constants(
                self.task_dispatches_bda,
                self.view_camera_bda,
                self.lod,
                geometry_data.draws_buffer.device_address.unwrap(),
                geometry_data.draws_count_buffer.device_address.unwrap(),
            );

            vk.cmd_push_constants(
                command_buffer,
                self.compute_prepass_pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                pc.data(),
            );

            vk.cmd_dispatch(command_buffer, geometry_data.mesh_instances_count, 1, 1);

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE);

            vk.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER, // src: the prepass
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[], // no buffer barriers
                &[], // no image barriers
            );

            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_prepass_dispatch_pipeline,
            );

            let pc = geometry_data.push_constants(
                self.task_dispatches_bda,
                self.view_camera_bda,
                self.lod,
                geometry_data.draws_buffer.device_address.unwrap(),
                geometry_data.draws_count_buffer.device_address.unwrap(),
            );

            vk.cmd_push_constants(
                command_buffer,
                self.compute_prepass_pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                pc.data(),
            );

            vk.cmd_dispatch(command_buffer, 1, 1, 1);

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(
                    vk::AccessFlags::INDIRECT_COMMAND_READ  // task_dispatches read by cmd_draw_mesh_tasks_indirect
                        | vk::AccessFlags::SHADER_READ, // draws buffer + draw_counter read by task/mesh shaders
                );

            vk.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER, // src: the prepass
                vk::PipelineStageFlags::DRAW_INDIRECT          // dst: fetch of indirect args
                    | vk::PipelineStageFlags::TASK_SHADER_EXT  // dst: task shader reads the draws buffer
                    | vk::PipelineStageFlags::MESH_SHADER_EXT, // dst: mesh shader reads it too
                vk::DependencyFlags::empty(),
                &[barrier],
                &[], // no buffer barriers
                &[], // no image barriers
            );
        }

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
                self.task_dispatches_bda,
                self.view_camera_bda,
                self.lod,
                geometry_data.draws_buffer.device_address.unwrap(),
                geometry_data.draws_count_buffer.device_address.unwrap(),
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
                self.task_dispatches_handle,
                0,
                1,
                std::mem::size_of::<vk::DrawMeshTasksIndirectCommandEXT>() as u32,
            );
        }
    }
}

fn create_compute_prepass_pipeline(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
    subgroup_size: u32,
) -> (vk::Pipeline, vk::PipelineLayout, vk::Pipeline) {
    let pc_range = [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::COMPUTE,
        offset: 0,
        size: std::mem::size_of::<gpu::PushConstants>() as u32,
    }];

    let set_layouts = [descriptor_set_layout];
    let push_constants_range = pc_range;
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);

    let pipeline_layout = unsafe {
        vk.create_pipeline_layout(&create_info, None)
            .expect("Failed to create pipeline layout")
    };

    let cs = &shaders::PREPASS_COMP;
    let cs_module = shaders::create_shader_module(vk, cs.spv).unwrap();
    let cs_name = unsafe { std::ffi::CStr::from_ptr(cs.entry_point_name()) };
    // TODO probably could hide this behind checking for VK_EXT_subgroup_size_control support or VK >= 1.3
    let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
        .required_subgroup_size(subgroup_size);
    let stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(cs_module)
        .name(cs_name)
        .push_next(&mut required);

    let create_info = vk::ComputePipelineCreateInfo::default()
        .layout(pipeline_layout)
        .stage(stage);

    let prepass_pipeline = unsafe {
        vk.create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .expect("Failed to create compute pipeline")[0]
    };

    unsafe {
        vk.destroy_shader_module(cs_module, None);
    }

    let cs = &shaders::PREPASS_SET_DISPATCHES_COMP;
    let cs_module = shaders::create_shader_module(vk, cs.spv).unwrap();
    let cs_name = unsafe { std::ffi::CStr::from_ptr(cs.entry_point_name()) };
    // TODO probably could hide this behind checking for VK_EXT_subgroup_size_control support or VK >= 1.3
    let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
        .required_subgroup_size(subgroup_size);
    let stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(cs_module)
        .name(cs_name)
        .push_next(&mut required);

    let create_info = vk::ComputePipelineCreateInfo::default()
        .layout(pipeline_layout)
        .stage(stage);

    let prepass_set_dispatches = unsafe {
        vk.create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .expect("Failed to create compute pipeline")[0]
    };

    unsafe {
        vk.destroy_shader_module(cs_module, None);
    }

    (prepass_pipeline, pipeline_layout, prepass_set_dispatches)
}
