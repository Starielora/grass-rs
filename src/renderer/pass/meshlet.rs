use crate::vkutils::{self, vk_destroy::VkDestroy};
use ash::vk;

pub struct MeshletPass {
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub render_target: vkutils::image::Image,
    pub depth_image: vkutils::image::Image,

    pipeline: vk::Pipeline,
    device: ash::Device,
}

impl MeshletPass {
    pub fn new(ctx: &mut vkutils::context::VulkanContext) -> Self {
        let command_buffers = ctx.graphics_command_pool.allocate_command_buffers(
            vk::CommandBufferLevel::PRIMARY,
            ctx.swapchain.images.len().try_into().unwrap(),
        );

        let extent = ctx.swapchain.extent;
        let pipeline_layout = ctx.bindless_descriptor_set.pipeline_layout;
        let format = ctx.swapchain.surface_format.format;

        let pipeline = create_pipeline(
            &ctx.device,
            &extent,
            pipeline_layout,
            format,
            ctx.depth_format,
        );

        let render_target = ctx.create_image(
            format,
            extent,
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::ImageAspectFlags::COLOR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let depth_image = ctx.create_image(
            ctx.depth_format,
            extent,
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::DEPTH,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        // LMAO WTF
        let ext_device = ash::ext::mesh_shader::Device::new(&ctx.instance, &ctx.device);

        for command_buffer in &command_buffers {
            record(
                &ctx.device,
                &ext_device,
                *command_buffer,
                (render_target.handle, render_target.view),
                (depth_image.handle, depth_image.view),
                extent,
                pipeline,
            );
        }

        Self {
            command_buffers,
            render_target,
            depth_image,
            pipeline,
            device: ctx.device.clone(),
        }
    }
}

impl std::ops::Drop for MeshletPass {
    fn drop(&mut self) {
        unsafe {
            self.render_target.vk_destroy();
            self.depth_image.vk_destroy();
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

fn record(
    device: &ash::Device,
    ext_device: &ash::ext::mesh_shader::Device,
    command_buffer: vk::CommandBuffer,
    color_image: (vk::Image, vk::ImageView),
    depth_image: (vk::Image, vk::ImageView),
    extent: vk::Extent2D,
    pipeline: vk::Pipeline,
) {
    let begin_info = vk::CommandBufferBeginInfo {
        ..Default::default()
    };
    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin command buffer");
    }

    record_image_barriers(&device, command_buffer, color_image.0, depth_image.0);

    begin_rendering(
        &device,
        command_buffer,
        color_image.1,
        depth_image.1,
        extent,
    );

    unsafe {
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
    }

    unsafe {
        ext_device.cmd_draw_mesh_tasks(command_buffer, 1, 1, 1);
    }

    unsafe {
        device.cmd_end_rendering(command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("Failed to end command buffer???");
    }
}

fn begin_rendering(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    color_image_view: vk::ImageView,
    depth_image_view: vk::ImageView,
    extent: vk::Extent2D,
) {
    let color_clear_value = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [153.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0, 1.0],
        },
    };

    let depth_clear_value = vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    };

    let color_attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(color_image_view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(color_clear_value)];

    let depth_attachment = vk::RenderingAttachmentInfo::default()
        .image_view(depth_image_view)
        .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(depth_clear_value);

    let rendering_info = vk::RenderingInfo::default()
        .render_area(vk::Rect2D {
            extent,
            offset: vk::Offset2D { x: 0, y: 0 },
        })
        .layer_count(1)
        .color_attachments(&color_attachments)
        .depth_attachment(&depth_attachment);

    unsafe {
        device.cmd_begin_rendering(command_buffer, &rendering_info);
    }
}

fn record_image_barriers(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    color_image: vk::Image,
    depth_image: vk::Image,
) {
    let color_subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(vk::REMAINING_ARRAY_LAYERS);

    vkutils::image_barrier(
        device,
        command_buffer,
        color_image,
        (
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ),
        color_subresource_range,
    );

    vkutils::image_barrier(
        device,
        command_buffer,
        depth_image,
        (
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        ),
        vkutils::depth_subresource_range(),
    );
}

fn create_pipeline(
    device: &ash::Device,
    extent: &vk::Extent2D,
    pipeline_layout: vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
) -> vk::Pipeline {
    // todo path lol
    let mut mesh_spv_file = std::fs::File::open("target/debug/meshlet.mesh.spv").unwrap();
    let mesh_spv = ash::util::read_spv(&mut mesh_spv_file).unwrap();
    let mesh_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&mesh_spv);
    let mesh_module = unsafe {
        device
            .create_shader_module(&mesh_shader_module_create_info, None)
            .unwrap()
    };
    let shader_main = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let mut fs_spv_file = std::fs::File::open("target/debug/meshlet.frag.spv").unwrap();
    let fs_spv = ash::util::read_spv(&mut fs_spv_file).unwrap();
    let fs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&fs_spv);
    let fs_module = unsafe {
        device
            .create_shader_module(&fs_shader_module_create_info, None)
            .unwrap()
    };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::MESH_EXT,
            module: mesh_module,
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

    let color_formats = [swapchain_format];

    let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_formats)
        .depth_attachment_format(depth_format);

    let create_info = vk::GraphicsPipelineCreateInfo::default()
        .push_next(&mut rendering_info)
        .stages(&shader_stages)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout);

    let pipelines = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .expect("Failed to create mesh shading pipeline.")
    };

    unsafe {
        device.destroy_shader_module(mesh_module, None);
        device.destroy_shader_module(fs_module, None);
    }

    pipelines[0]
}
