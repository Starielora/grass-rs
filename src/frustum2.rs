use ash::vk;

use crate::{
    meshlet2::gpu::{self, CPUPushConstant},
    vkutils::shaders,
};

pub struct Frustum2 {
    vk: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline_solid: vk::Pipeline,
    pipeline_wireframe: vk::Pipeline,
    view_camera: vk::DeviceAddress,
    cull_camera: vk::DeviceAddress,
    pub planes_color: [f32; 4],
    pub edges_color: [f32; 4],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PushConstant {
    view_camera: vk::DeviceAddress,
    cull_camera: vk::DeviceAddress,
    edges_color: glm::Vec4,
    planes_color: glm::Vec4,
}

const _: () = assert!(std::mem::size_of::<PushConstant>() <= 128);

impl gpu::CPUPushConstant for PushConstant {
    fn stage_flags() -> vk::ShaderStageFlags {
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
    }
}

impl Frustum2 {
    pub fn new(
        vk: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
        swapchain_format: vk::Format,
        depth_format: vk::Format,
        view_camera: vk::DeviceAddress,
        cull_camera: vk::DeviceAddress,
    ) -> Self {
        let pipeline_layout =
            gpu::create_pipeline_layout(vk, descriptor_set_layout, PushConstant::range());
        let pipeline_solid = create_graphics_pipeline(
            &vk,
            pipeline_layout,
            swapchain_format,
            depth_format,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            false,
            vk::FALSE,
        );
        let pipeline_wireframe = create_graphics_pipeline(
            &vk,
            pipeline_layout,
            swapchain_format,
            depth_format,
            vk::PrimitiveTopology::LINE_LIST,
            true,
            vk::TRUE,
        );

        Self {
            vk: vk.clone(),
            pipeline_layout,
            pipeline_solid,
            pipeline_wireframe,
            view_camera,
            cull_camera,
            planes_color: [1.0, 1.0, 1.0, 0.25],
            edges_color: [1.0, 1.0, 0.0, 1.0],
        }
    }

    pub fn record(&self, command_buffer: vk::CommandBuffer, extent: vk::Extent2D) {
        let vk = &self.vk;
        let pc = PushConstant {
            view_camera: self.view_camera,
            cull_camera: self.cull_camera,
            edges_color: self.edges_color.into(),
            planes_color: self.planes_color.into(),
        };

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

        unsafe {
            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                PushConstant::stage_flags(),
                0,
                pc.data(),
            );

            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_solid,
            );
            vk.cmd_set_viewport(command_buffer, 0, &[viewport]);
            vk.cmd_set_scissor(command_buffer, 0, &[scissors]);
            vk.cmd_draw(command_buffer, 36, 1, 0, 0);

            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_wireframe,
            );
            vk.cmd_set_viewport(command_buffer, 0, &[viewport]);
            vk.cmd_set_scissor(command_buffer, 0, &[scissors]);
            vk.cmd_draw(command_buffer, 26, 1, 0, 0);
        }
    }
}

fn create_graphics_pipeline(
    vk: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
    topology: vk::PrimitiveTopology,
    is_wireframe: bool,
    depth_write: vk::Bool32,
) -> vk::Pipeline {
    let vs = &shaders::FRUSTUM_VERT;
    let fs = &shaders::FRUSTUM_FRAG;

    let vs_module = shaders::create_shader_module(vk, vs.spv).unwrap();
    let fs_module = shaders::create_shader_module(vk, fs.spv).unwrap();

    let spec_entry = vk::SpecializationMapEntry {
        constant_id: 0,
        offset: 0,
        size: std::mem::size_of::<u32>(),
    };
    let spec_data: u32 = if is_wireframe { 1 } else { 0 };
    let spec_data_bytes = spec_data.to_ne_bytes();
    let spec_info = vk::SpecializationInfo {
        map_entry_count: 1,
        p_map_entries: &spec_entry,
        data_size: spec_data_bytes.len(),
        p_data: spec_data_bytes.as_ptr() as *const std::ffi::c_void,
        ..Default::default()
    };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vs_module,
            p_name: vs.entry_point_name(),
            p_specialization_info: &spec_info,
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: fs_module,
            p_name: fs.entry_point_name(),
            ..Default::default()
        },
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
        topology,
        ..Default::default()
    };

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::NONE,
        front_face: vk::FrontFace::CLOCKWISE,
        depth_bias_enable: vk::FALSE,
        depth_bias_constant_factor: 1.25,
        depth_bias_clamp: 0.0,
        depth_bias_slope_factor: 1.75,
        line_width: if is_wireframe { 4.0 } else { 1.0 },
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
        depth_write_enable: depth_write,
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
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .dynamic_state(&dynamic_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout);

    let pipelines = unsafe {
        vk.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .unwrap()
    };

    unsafe {
        vk.destroy_shader_module(vs_module, None);
        vk.destroy_shader_module(fs_module, None);
    }

    pipelines[0]
}

impl std::ops::Drop for Frustum2 {
    fn drop(&mut self) {
        let vk = &self.vk;
        unsafe {
            vk.destroy_pipeline(self.pipeline_solid, None);
            vk.destroy_pipeline(self.pipeline_wireframe, None);
            vk.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
