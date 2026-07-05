use ash::vk;

use crate::{meshlet2::PushConstants, vkutils::shaders};

pub fn create_pipeline(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let pipeline_layout = create_pipeline_layout(vk, descriptor_set_layout);

    let ms = &shaders::MESHLET_MESH;
    let ts = &shaders::MESHLET_TASK;
    let fs = &shaders::MESHLET_FRAG;

    // TODO error handling
    let ms_module = shaders::create_shader_module(vk, ms.spv).unwrap();
    let ts_module = shaders::create_shader_module(vk, ts.spv).unwrap();
    let fs_module = shaders::create_shader_module(vk, fs.spv).unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::TASK_EXT,
            module: ts_module,
            p_name: ts.entry_point_name(),
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::MESH_EXT,
            module: ms_module,
            p_name: ms.entry_point_name(),
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: fs_module,
            p_name: fs.entry_point_name(),
            ..Default::default()
        },
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
fn get_push_constants_stage_flags() -> vk::ShaderStageFlags {
    vk::ShaderStageFlags::MESH_EXT | vk::ShaderStageFlags::TASK_EXT | vk::ShaderStageFlags::FRAGMENT
}

fn get_push_constant_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: get_push_constants_stage_flags(),
        offset: 0,
        size: std::mem::size_of::<PushConstants>() as u32,
    }]
}

fn create_pipeline_layout(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let set_layouts = [descriptor_set_layout];
    let push_constants_range = get_push_constant_range();
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);
    unsafe {
        vk.create_pipeline_layout(&create_info, None)
            .expect("Failed to create pipeline layout")
    }
}
