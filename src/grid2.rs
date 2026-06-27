use ash::vk;

use crate::vkutils;
use crate::vkutils::shaders;

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct PushConstants {
    view_camera: vk::DeviceAddress,
}

pub struct Grid2 {
    pipeline: vk::Pipeline,
    vk: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    push_constants: PushConstants,
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
        vkctx: &vkutils::context::VulkanContext,
        view_camera: vk::DeviceAddress,
    ) -> Result<Grid2, Box<dyn std::error::Error>> {
        let device = &vkctx.device;
        let window_extent = &vkctx.swapchain.extent;
        let swapchain_format = vkctx.swapchain.surface_format.format;
        let depth_format = vkctx.depth_format;

        let pipeline_layout = create_pipeline_layout(vkctx.bindless_descriptor_set.layout, device);

        let vs = &shaders::GRID_VERT;
        let fs = &shaders::GRID_FRAG;

        let vs_module = shaders::create_shader_module(device, vs.spv)?;
        let fs_module = shaders::create_shader_module(device, fs.spv)?;

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
            vk: device.clone(),
            pipeline_layout,
            push_constants: PushConstants {
                view_camera: view_camera,
            },
        })
    }

    fn push_constants_data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (&self.push_constants as *const PushConstants) as *const u8,
                std::mem::size_of::<PushConstants>(),
            )
        }
    }

    pub fn record(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            let vk = &self.vk;

            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                self.push_constants_data(),
            );

            vk.cmd_draw(command_buffer, 6, 1, 0, 0);
        }
    }
}

fn get_push_constant_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: std::mem::size_of::<PushConstants>() as u32,
    }]
}

fn create_pipeline_layout(
    descriptor_set_layout: vk::DescriptorSetLayout,
    vk: &ash::Device,
) -> vk::PipelineLayout {
    let set_layouts = [descriptor_set_layout];
    let push_constants_range = get_push_constant_range();
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);
    unsafe {
        vk.create_pipeline_layout(&create_info, None)
            .expect("Failed to create traditional pipeline layout")
    }
}
