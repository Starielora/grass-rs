use ash::vk;

use crate::vkutils::{self, push_constants::GPUPushConstants, vk_destroy::VkDestroy};

pub struct DepthMapDisplayPass {
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub render_target: vkutils::image::Image,
    pipeline: vk::Pipeline,
    device: ash::Device,
}

impl std::ops::Drop for DepthMapDisplayPass {
    fn drop(&mut self) {
        self.render_target.vk_destroy();
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl DepthMapDisplayPass {
    pub fn new(
        ctx: &mut vkutils::context::VulkanContext,
        src_depth_map: (vk::Image, vk::ImageView, vk::ImageLayout),
        sampler: vk::Sampler,
    ) -> Self {
        let command_buffers = ctx.graphics_command_pool.allocate_command_buffers(
            vk::CommandBufferLevel::PRIMARY,
            ctx.swapchain.images.len().try_into().unwrap(),
        );

        let extent = ctx.swapchain.extent;
        let pipeline_layout = ctx.bindless_descriptor_set.pipeline_layout;
        let pipeline = create_pipeline(
            &ctx.device,
            &extent,
            pipeline_layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
        );

        let depth_display_render_target = ctx.create_image(
            ctx.swapchain.surface_format.format,
            ctx.swapchain.extent,
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::ImageAspectFlags::COLOR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let resource_id: u32 = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        ctx.bindless_descriptor_set.update_sampler2d(
            src_depth_map.1,
            sampler,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            resource_id,
        );

        for command_buffer in &command_buffers {
            unsafe {
                let begin_info = vk::CommandBufferBeginInfo::default();
                ctx.device
                    .begin_command_buffer(*command_buffer, &begin_info)
                    .expect("Failed to begin command buffer");
            }

            ctx.bindless_descriptor_set
                .cmd_bind(*command_buffer, vk::PipelineBindPoint::GRAPHICS);

            record(
                &ctx.device,
                *command_buffer,
                pipeline,
                ctx.bindless_descriptor_set.pipeline_layout,
                // TODO double buffering
                src_depth_map,
                (
                    depth_display_render_target.handle,
                    depth_display_render_target.view,
                ),
                ctx.swapchain.extent,
                resource_id,
            );

            unsafe {
                ctx.device
                    .end_command_buffer(*command_buffer)
                    .expect("Failed to end command buffer");
            }
        }

        Self {
            command_buffers,
            render_target: depth_display_render_target,
            pipeline,
            device: ctx.device.clone(),
        }
    }
}

fn record(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    src_depth_image: (vk::Image, vk::ImageView, vk::ImageLayout),
    color_image: (vk::Image, vk::ImageView),
    extent: vk::Extent2D,
    sampler_id: u32,
) {
    vkutils::image_barrier(
        &device,
        command_buffer,
        color_image.0,
        (
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::SHADER_WRITE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        vkutils::color_subresource_range(),
    );

    vkutils::image_barrier(
        &device,
        command_buffer,
        src_depth_image.0,
        (
            src_depth_image.2,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        vkutils::depth_subresource_range(),
    );

    let color_attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(color_image.1)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        })];

    let rendering_info = vk::RenderingInfo::default()
        .render_area(vk::Rect2D {
            extent,
            offset: vk::Offset2D { x: 0, y: 0 },
        })
        .layer_count(1)
        .color_attachments(&color_attachments);

    let mut push_constants = GPUPushConstants::default();
    push_constants.depth_sampler_index = sampler_id;

    unsafe {
        device.cmd_push_constants(
            command_buffer,
            pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            std::slice::from_raw_parts(
                (&push_constants as *const GPUPushConstants) as *const u8,
                std::mem::size_of::<GPUPushConstants>(),
            ),
        );
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
        device.cmd_begin_rendering(command_buffer, &rendering_info);
        device.cmd_draw(command_buffer, 3, 1, 0, 0);
        device.cmd_end_rendering(command_buffer);
    }
}

fn create_pipeline(
    device: &ash::Device,
    extent: &vk::Extent2D,
    pipeline_layout: vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
) -> vk::Pipeline {
    // todo path lol
    let mut vs_spv_file = std::fs::File::open("target/debug/depth_display.vert.spv").unwrap();
    let vs_spv = ash::util::read_spv(&mut vs_spv_file).unwrap();
    let vs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
    let vs_module = unsafe {
        device
            .create_shader_module(&vs_shader_module_create_info, None)
            .unwrap()
    };
    let shader_main = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let mut fs_spv_file = std::fs::File::open("target/debug/depth_display.frag.spv").unwrap();
    let fs_spv = ash::util::read_spv(&mut fs_spv_file).unwrap();
    let fs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&fs_spv);
    let fs_module = unsafe {
        device
            .create_shader_module(&fs_shader_module_create_info, None)
            .unwrap()
    };

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

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let viewport = vk::Viewport {
        width: extent.width as f32,
        height: extent.height as f32,
        max_depth: 1.0,
        ..Default::default()
    };

    let scissors = vk::Rect2D {
        extent: *extent,
        ..Default::default()
    };

    let viewports = [viewport];
    let scissors = [scissors];
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::NONE,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        depth_bias_enable: vk::FALSE,
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

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default();

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

    pipelines[0]
}
