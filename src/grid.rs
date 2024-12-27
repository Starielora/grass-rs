use std::ffi::CStr;

use ash::vk;

use crate::{
    drawable,
    push_constants::{get_push_constants_range, GPUPushConstants},
};

pub struct Grid {
    pipeline: vk::Pipeline,
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
}

impl Grid {
    pub fn new(
        device: &ash::Device,
        window_extent: &vk::Extent2D,
        swapchain_format: vk::Format,
        depth_format: vk::Format,
    ) -> Result<Grid, Box<dyn std::error::Error>> {
        let shader_main = CStr::from_bytes_with_nul(b"main\0")?;

        let mut vs_spv_file = std::fs::File::open("target/debug/grid.vert.spv")?;
        let vs_spv = ash::util::read_spv(&mut vs_spv_file)?;
        let vs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
        let vs_module =
            unsafe { device.create_shader_module(&vs_shader_module_create_info, None) }?;

        let mut fs_spv_file = std::fs::File::open("target/debug/grid.frag.spv")?;
        let fs_spv = ash::util::read_spv(&mut fs_spv_file)?;
        let fs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&fs_spv);
        let fs_module =
            unsafe { device.create_shader_module(&fs_shader_module_create_info, None) }?;

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: vs_module,
                p_name: shader_main.as_ptr(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: fs_module,
                p_name: shader_main.as_ptr(),
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

        let viewport = vk::Viewport {
            width: window_extent.width as f32,
            height: window_extent.height as f32,
            max_depth: 1.0,
            ..Default::default()
        };

        let scissors = vk::Rect2D {
            extent: *window_extent,
            ..Default::default()
        };

        let viewports = [viewport];
        let scissors = [scissors];
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
            line_width: 1.0,
            ..Default::default()
        };

        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: vk::FALSE,
            min_sample_shading: 1.0,
            alpha_to_coverage_enable: vk::FALSE,
            alpha_to_one_enable: vk::FALSE,
            ..Default::default()
        };

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: vk::TRUE,
            depth_write_enable: vk::TRUE,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
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

        let layouts = [];
        let push_constants_range = get_push_constants_range();
        let create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&layouts)
            .push_constant_ranges(&push_constants_range);
        let pipeline_layout = unsafe { device.create_pipeline_layout(&create_info, None).unwrap() };

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
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout);

        let pipelines = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .unwrap()
        };

        unsafe {
            device.destroy_shader_module(vs_module, None);
            device.destroy_shader_module(fs_module, None);
        }

        Ok(Self {
            pipeline: pipelines[0],
            device: device.clone(),
            pipeline_layout,
        })
    }
}

impl drawable::Drawable for Grid {
    fn cmd_draw(
        self: &mut Self,
        command_buffer: &vk::CommandBuffer,
        push_constants: &GPUPushConstants,
    ) {
        unsafe {
            self.device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            self.device.cmd_push_constants(
                *command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            self.device.cmd_draw(*command_buffer, 6, 1, 0, 0);
        }
    }
}

impl std::ops::Drop for Grid {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}
