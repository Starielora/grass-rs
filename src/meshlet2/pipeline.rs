use ash::vk;

use crate::{
    meshlet2::gpu::{self, CPUPushConstant},
    vkutils::shaders,
};

pub fn create_pipeline(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
    subgroup_size: u32,
) -> (vk::Pipeline, vk::PipelineLayout) {
    assert!(subgroup_size <= 64, "TaskPayload array is sized [64]");
    let pipeline_layout =
        gpu::create_pipeline_layout(vk, descriptor_set_layout, super::PushConstant::range());

    let ms = &shaders::MESHLET_MESH;
    let ts = &shaders::MESHLET_TASK;
    let fs = &shaders::MESHLET_FRAG;

    // TODO error handling
    let ms_module = shaders::create_shader_module(vk, ms.spv).unwrap();
    let ts_module = shaders::create_shader_module(vk, ts.spv).unwrap();
    let fs_module = shaders::create_shader_module(vk, fs.spv).unwrap();
    let ts_name = unsafe { std::ffi::CStr::from_ptr(ts.entry_point_name()) };
    let ms_name = unsafe { std::ffi::CStr::from_ptr(ms.entry_point_name()) };
    let fs_name = unsafe { std::ffi::CStr::from_ptr(fs.entry_point_name()) };

    let spec_entry = vk::SpecializationMapEntry {
        constant_id: 0,
        offset: 0,
        size: std::mem::size_of::<u32>(),
    };
    let spec_data: u32 = subgroup_size;
    let spec_data_bytes = spec_data.to_ne_bytes();
    let spec_info = vk::SpecializationInfo {
        map_entry_count: 1,
        p_map_entries: &spec_entry,
        data_size: spec_data_bytes.len(),
        p_data: spec_data_bytes.as_ptr() as *const std::ffi::c_void,
        ..Default::default()
    };

    // TODO probably could hide this behind checking for VK_EXT_subgroup_size_control support or VK >= 1.3
    let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
        .required_subgroup_size(subgroup_size);

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
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        depth_bias_enable: vk::FALSE,
        depth_bias_constant_factor: 1.25,
        depth_bias_clamp: 0.0,
        depth_bias_slope_factor: 1.75,
        line_width: 1.0,
        ..Default::default()
    };

    let multisample_state = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_8,
        sample_shading_enable: vk::FALSE,
        min_sample_shading: 1.0,
        alpha_to_coverage_enable: vk::FALSE,
        alpha_to_one_enable: vk::FALSE,
        ..Default::default()
    };

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: vk::TRUE,
        depth_write_enable: vk::TRUE,
        depth_compare_op: vk::CompareOp::GREATER,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        ..Default::default()
    };

    let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::TRUE,
        src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    };

    let attachments = [color_blend_attachment_state];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

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
            .expect("Failed to create mesh shading pipeline.")
    };

    unsafe {
        vk.destroy_shader_module(ts_module, None);
        vk.destroy_shader_module(ms_module, None);
        vk.destroy_shader_module(fs_module, None);
    }

    (pipelines[0], pipeline_layout)
}
