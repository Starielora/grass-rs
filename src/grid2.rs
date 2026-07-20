use ash::vk;

use crate::{
    meshlet2::gpu::{self, CPUPushConstant},
    vkutils::shaders,
};

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct PushConstant {
    view_camera: vk::DeviceAddress,
}

impl gpu::CPUPushConstant for PushConstant {
    fn stage_flags() -> vk::ShaderStageFlags {
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
    }
}

pub struct Grid2 {
    vk: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl std::ops::Drop for Grid2 {
    fn drop(&mut self) {
        unsafe {
            self.vk.destroy_pipeline_layout(self.pipeline_layout, None);
            self.vk.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl Grid2 {
    pub fn new(
        vk: &ash::Device,
        surface_format: vk::Format,
        depth_format: vk::Format,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<Grid2, Box<dyn std::error::Error>> {
        let pipeline_layout = create_pipeline_layout(vk, descriptor_set_layout);

        let vs = &shaders::GRID_VERT;
        let fs = &shaders::GRID_FRAG;

        let vs_module = shaders::create_shader_module(vk, vs.spv)?;
        let fs_module = shaders::create_shader_module(vk, fs.spv)?;

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: vs_module,
                p_name: vs.entry_point_name(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: fs_module,
                p_name: fs.entry_point_name(),
                ..Default::default()
            },
        ];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            ..Default::default()
        };

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
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

        let color_formats = [surface_format];

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

        Ok(Self {
            vk: vk.clone(),
            pipeline: pipelines[0],
            pipeline_layout,
        })
    }

    pub fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
        view_camera: vk::DeviceAddress,
    ) {
        unsafe {
            let vk = &self.vk;
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

            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let pc = PushConstant { view_camera };
            vk.cmd_set_viewport(command_buffer, 0, &[viewport]);
            vk.cmd_set_scissor(command_buffer, 0, &[scissors]);
            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                PushConstant::stage_flags(),
                0,
                pc.data(),
            );

            vk.cmd_draw(command_buffer, 6, 1, 0, 0);
        }
    }
}

fn get_push_constans_stage_flags() -> vk::ShaderStageFlags {
    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
}

fn create_pipeline_layout(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let set_layouts = [descriptor_set_layout];
    let push_constants_range = [PushConstant::range()];
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);
    unsafe {
        vk.create_pipeline_layout(&create_info, None)
            .expect("Failed to create traditional pipeline layout")
    }
}
