use ash::vk;

use crate::{vkutils, vkutils_new};

// TODO this is the same shit as mesh::pipeline
pub struct DepthMapDisplayPipeline {
    device: ash::Device,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    sampler: vk::Sampler,
}

impl DepthMapDisplayPipeline {
    pub fn new(ctx: &vkutils::Context, depth_image_view: vk::ImageView) -> Self {
        let pipeline_layout = ctx.bindless_descriptor_set.pipeline_layout;

        let pipeline = create_graphics_pipeline(
            &ctx.device,
            &ctx.swapchain.extent,
            &pipeline_layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_image.format,
        );

        let sampler = create_depth_map_sampler(&ctx.device);
        // TODO this update should be done globally, since descriptor set is global
        update_bindless_descriptor_set(
            &ctx.device,
            depth_image_view,
            sampler,
            ctx.bindless_descriptor_set.handle,
        );

        Self {
            device: ctx.device.clone(),
            pipeline_layout,
            pipeline,
            sampler,
        }
    }

    pub fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        descriptor_set: vk::DescriptorSet,
        src_depth_image: (vk::Image, vk::ImageView),
        color_image: (vk::Image, vk::ImageView),
        target_image: (vk::Image, vk::ImageView),
        extent: vk::Extent2D,
    ) {
        let device = self.device.clone();
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();
            device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");
        }

        vkutils_new::image_barrier(
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
            vkutils_new::color_subresource_range(),
        );

        vkutils_new::image_barrier(
            &device,
            command_buffer,
            target_image.0,
            (
                vk::ImageLayout::UNDEFINED,
                vk::AccessFlags::NONE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
            ),
            (
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            vkutils_new::color_subresource_range(),
        );

        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
            .resolve_image_view(target_image.1)
            .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(color_image.1)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            })];

        let depth_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(src_depth_image.1)
            .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::NONE);

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                extent,
                offset: vk::Offset2D { x: 0, y: 0 },
            })
            .layer_count(1)
            .color_attachments(&color_attachments)
            .depth_attachment(&depth_attachment);

        unsafe {
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            device.cmd_begin_rendering(command_buffer, &rendering_info);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_rendering(command_buffer);

            vkutils_new::image_barrier(
                &device,
                command_buffer,
                target_image.0,
                (
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (
                    vk::ImageLayout::PRESENT_SRC_KHR,
                    vk::AccessFlags::NONE,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                ),
                vkutils_new::color_subresource_range(),
            );

            device
                .end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");
        }
    }
}

impl std::ops::Drop for DepthMapDisplayPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

fn create_graphics_pipeline(
    device: &ash::Device,
    window_extent: &vk::Extent2D,
    pipeline_layout: &vk::PipelineLayout,
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
        .layout(*pipeline_layout);

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

fn create_depth_map_sampler(device: &ash::Device) -> vk::Sampler {
    let create_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .mip_lod_bias(0.0)
        .max_anisotropy(1.0)
        .min_lod(0.0)
        .max_lod(1.0)
        .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE);

    unsafe {
        device
            .create_sampler(&create_info, None)
            .expect("Failed to create depth map sampler.")
    }
}

fn update_bindless_descriptor_set(
    device: &ash::Device,
    depth_image: vk::ImageView,
    sampler: vk::Sampler,
    descriptor_set: vk::DescriptorSet,
) {
    let descriptor_image_info = [vk::DescriptorImageInfo::default()
        .sampler(sampler)
        .image_view(depth_image)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

    let descriptor_writes = [vk::WriteDescriptorSet::default()
        .dst_set(descriptor_set)
        .dst_binding(vkutils_new::descriptor_set::bindless::DEPTH_SAMPLER_BINDING)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .dst_array_element(0) // TODO resource_id
        .image_info(&descriptor_image_info)];

    let descriptor_copies = [];
    unsafe { device.update_descriptor_sets(&descriptor_writes, &descriptor_copies) };
}
