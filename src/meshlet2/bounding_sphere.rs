use ash::vk;

use crate::{
    meshlet2::{gpu, GeometryBuffers},
    vkutils::shaders,
};

enum DrawMode {
    NONE,
    MESH,
    MESHLET,
}

pub struct BoundingSphere {
    vk: ash::Device,
    vk_ext: ash::ext::mesh_shader::Device,
    pipeline_mesh: vk::Pipeline,
    pipeline_meshlet: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    view_camera_bda: vk::DeviceAddress,
    subgroup_size: u32,
    max_task_workgroup_count: [u32; 3],
    draw_mode: DrawMode,
}

impl BoundingSphere {
    pub fn new(
        vk: &ash::Device,
        vk_ext: &ash::ext::mesh_shader::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
        swapchain_format: vk::Format,
        depth_format: vk::Format,
        view_camera: vk::DeviceAddress,
        subgroup_size: u32,
        max_task_workgroup_count: [u32; 3],
    ) -> Self {
        let pipeline_layout = create_pipeline_layout(vk, descriptor_set_layout);
        let pipeline_mesh = create_pipeline(
            vk,
            pipeline_layout,
            swapchain_format,
            depth_format,
            subgroup_size,
            true,
        );

        let pipeline_meshlet = create_pipeline(
            vk,
            pipeline_layout,
            swapchain_format,
            depth_format,
            subgroup_size,
            false,
        );

        Self {
            vk: vk.clone(),
            vk_ext: vk_ext.clone(),
            pipeline_mesh,
            pipeline_meshlet,
            pipeline_layout,
            view_camera_bda: view_camera,
            subgroup_size,
            max_task_workgroup_count,
            draw_mode: DrawMode::NONE,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.draw_mode = match self.draw_mode {
            DrawMode::NONE => DrawMode::MESH,
            DrawMode::MESH => DrawMode::MESHLET,
            DrawMode::MESHLET => DrawMode::NONE,
        };
    }

    pub fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
        geometry_data: &GeometryBuffers,
    ) {
        let vk = &self.vk;
        let vk_ext = &self.vk_ext;

        let (pipeline, dispatch_count_x, dispatch_count_y) = match self.draw_mode {
            DrawMode::NONE => return,
            DrawMode::MESH => {
                let total_groups = (geometry_data.mesh_instances_count + (self.subgroup_size - 1))
                    / self.subgroup_size;
                let max_dim = self.max_task_workgroup_count[0]; // maxTaskWorkGroupCount[0] — ideally query, don't hardcode
                let group_x = total_groups.min(max_dim);
                let group_y = (total_groups + max_dim - 1) / max_dim;

                // TODO 2D dispatch similarly to meshlets
                (self.pipeline_mesh, group_x, group_y)
            }
            DrawMode::MESHLET => {
                let total_groups = (geometry_data.meshlet_instances_count
                    + (self.subgroup_size - 1))
                    / self.subgroup_size;
                let max_dim = self.max_task_workgroup_count[0]; // maxTaskWorkGroupCount[0] — ideally query, don't hardcode
                let group_x = total_groups.min(max_dim);
                let group_y = (total_groups + max_dim - 1) / max_dim;
                (self.pipeline_meshlet, group_x, group_y)
            }
        };

        unsafe {
            vk.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
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

            vk_ext.cmd_draw_mesh_tasks(command_buffer, dispatch_count_x, dispatch_count_y, 1);
        }
    }
}

impl std::ops::Drop for BoundingSphere {
    fn drop(&mut self) {
        let vk = &self.vk;
        unsafe {
            vk.destroy_pipeline(self.pipeline_mesh, None);
            vk.destroy_pipeline(self.pipeline_meshlet, None);
            vk.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

fn create_pipeline(
    vk: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
    subgroup_size: u32,
    is_mesh: bool,
) -> vk::Pipeline {
    assert!(subgroup_size <= 64, "TaskPayload array is sized [64]");

    let ms = &shaders::BOUNDING_SPHERE_MESH;
    let ts = &shaders::BOUNDING_SPHERE_TASK;
    let fs = &shaders::MESHLET_FRAG;

    // TODO error handling
    let ms_module = shaders::create_shader_module(vk, ms.spv).unwrap();
    let ts_module = shaders::create_shader_module(vk, ts.spv).unwrap();
    let fs_module = shaders::create_shader_module(vk, fs.spv).unwrap();
    let ts_name = unsafe { std::ffi::CStr::from_ptr(ts.entry_point_name()) };
    let ms_name = unsafe { std::ffi::CStr::from_ptr(ms.entry_point_name()) };
    let fs_name = unsafe { std::ffi::CStr::from_ptr(fs.entry_point_name()) };

    // TODO probably could hide this behind checking for VK_EXT_subgroup_size_control support or VK >= 1.3
    let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
        .required_subgroup_size(subgroup_size);

    let spec_entries = [
        vk::SpecializationMapEntry {
            constant_id: 0,
            offset: 0,
            size: std::mem::size_of::<u32>(),
        },
        vk::SpecializationMapEntry {
            constant_id: 1,
            offset: std::mem::size_of::<u32>() as u32,
            size: std::mem::size_of::<vk::Bool32>(),
        },
    ];
    let spec_data: [u32; 2] = [subgroup_size, is_mesh as u32];
    let spec_data_u8: [u8; 8] = unsafe { std::mem::transmute(spec_data) }; // TODO fragile af
    let spec_info = vk::SpecializationInfo::default()
        .map_entries(&spec_entries)
        .data(&spec_data_u8);

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::TASK_EXT)
            .module(ts_module)
            .name(ts_name)
            .specialization_info(&spec_info)
            .push_next(&mut required),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::MESH_EXT)
            .module(ms_module)
            .name(ms_name)
            .specialization_info(&spec_info),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fs_module)
            .name(fs_name),
    ];

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        line_width: 1.5,
        ..Default::default()
    };

    let multisample_state = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_8,
        min_sample_shading: 1.0,
        ..Default::default()
    };

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: vk::TRUE,
        depth_write_enable: vk::FALSE,
        depth_compare_op: vk::CompareOp::GREATER,
        ..Default::default()
    };

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::TRUE,
        src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    };
    let attachments = [color_blend_attachment];
    let color_blend_state =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachments);

    let color_formats = [swapchain_format];
    let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_formats)
        .depth_attachment_format(depth_format);

    let create_info = vk::GraphicsPipelineCreateInfo::default()
        .push_next(&mut rendering_info)
        .stages(&shader_stages)
        .viewport_state(&viewport_state)
        .dynamic_state(&dynamic_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout);

    let pipelines = unsafe {
        vk.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .expect("Failed to create meshlet bounding sphere pipeline")
    };

    unsafe {
        vk.destroy_shader_module(ts_module, None);
        vk.destroy_shader_module(ms_module, None);
        vk.destroy_shader_module(fs_module, None);
    }

    pipelines[0]
}

// TODO duplicate with graphics pipeline, because they're using the same task shader.
fn create_pipeline_layout(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let set_layouts = [descriptor_set_layout];
    let push_constants_range = gpu::get_push_constant_range();
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);
    unsafe {
        vk.create_pipeline_layout(&create_info, None)
            .expect("Failed to create pipeline layout")
    }
}
