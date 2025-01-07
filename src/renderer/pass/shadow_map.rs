use crate::vkutils::push_constants::GPUPushConstants;
use crate::vkutils::vk_destroy::VkDestroy;
use crate::{mesh, vkutils};
use ash::vk;

pub struct ShadowMapPass {
    pub command_buffers: [vk::CommandBuffer; 2],
    pub output_depth_image: vkutils::image::Image,

    pipeline: vk::Pipeline,
    device: ash::Device,
}

impl std::ops::Drop for ShadowMapPass {
    fn drop(&mut self) {
        self.output_depth_image.vk_destroy();
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl ShadowMapPass {
    pub fn new(
        ctx: &mut vkutils::context::VulkanContext,
        light_pov_camera_buffer_device_address: vk::DeviceAddress,
        meshes: &[mesh::Mesh],
    ) -> Self {
        let command_buffers = ctx
            .graphics_command_pool
            .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 2);

        let command_buffers = [command_buffers[0], command_buffers[1]];

        let depth_image = ctx.create_image(
            ctx.depth_format,
            ctx.swapchain.extent,
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::DEPTH,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let extent = ctx.swapchain.extent;
        let pipeline_layout = ctx.bindless_descriptor_set.pipeline_layout;
        let pipeline = create_pipeline(&ctx.device, &extent, pipeline_layout, ctx.depth_format);

        for command_buffer in command_buffers {
            record(
                &ctx.device,
                command_buffer,
                pipeline,
                pipeline_layout,
                extent,
                (depth_image.handle, depth_image.view),
                light_pov_camera_buffer_device_address,
                meshes,
            );
        }

        Self {
            command_buffers,
            output_depth_image: depth_image,
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
    extent: vk::Extent2D,
    light_pov_depth_image: (vk::Image, vk::ImageView),
    light_camera_data_buffer_address: vk::DeviceAddress,
    meshes: &[mesh::Mesh],
) {
    let (camera_pov_depth_image, camera_pov_depth_image_view) = light_pov_depth_image;

    let mut push_constants = GPUPushConstants::default();
    push_constants.camera_data_buffer_address = light_camera_data_buffer_address;

    let begin_info = vk::CommandBufferBeginInfo::default();
    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin command buffer.");
    }

    vkutils::image_barrier(
        &device,
        command_buffer,
        camera_pov_depth_image,
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

    let depth_attachment = vk::RenderingAttachmentInfo::default()
        .image_view(camera_pov_depth_image_view)
        .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        });

    let rendering_info = vk::RenderingInfo::default()
        .render_area(vk::Rect2D {
            extent,
            offset: vk::Offset2D { x: 0, y: 0 },
        })
        .layer_count(1)
        .depth_attachment(&depth_attachment);

    unsafe {
        device.cmd_begin_rendering(command_buffer, &rendering_info);
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
    }

    for mesh in meshes {
        mesh.cmd_draw(
            &device,
            command_buffer,
            pipeline_layout,
            &mut push_constants,
        );
    }

    unsafe {
        device.cmd_end_rendering(command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("Failed to end command buffer");
    }
}

fn create_pipeline(
    device: &ash::Device,
    extent: &vk::Extent2D,
    pipeline_layout: vk::PipelineLayout,
    depth_format: vk::Format,
) -> vk::Pipeline {
    // todo path lol
    let mut vs_spv_file = std::fs::File::open("target/debug/cube.vert.spv").unwrap();
    let vs_spv = ash::util::read_spv(&mut vs_spv_file).unwrap();
    let vs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
    let vs_module = unsafe {
        device
            .create_shader_module(&vs_shader_module_create_info, None)
            .unwrap()
    };
    let shader_main = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let shader_stages = [vk::PipelineShaderStageCreateInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        module: vs_module,
        p_name: shader_main.as_ptr(),
        ..Default::default()
    }];

    let vertex_binding_desciptions = [vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride((std::mem::size_of::<f32>() * 8) as u32)
        .input_rate(vk::VertexInputRate::VERTEX)];

    let vertex_attribute_descriptions = [
        vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0),
        vk::VertexInputAttributeDescription::default()
            .location(1)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset((std::mem::size_of::<f32>() * 3) as u32),
        vk::VertexInputAttributeDescription::default()
            .location(2)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((std::mem::size_of::<f32>() * 6) as u32),
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&vertex_binding_desciptions)
        .vertex_attribute_descriptions(&vertex_attribute_descriptions);

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
        cull_mode: vk::CullModeFlags::BACK,
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

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: vk::TRUE,
        depth_write_enable: vk::TRUE,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        ..Default::default()
    };

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    let mut rendering_info =
        vk::PipelineRenderingCreateInfo::default().depth_attachment_format(depth_format);

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
    }

    pipelines[0]
}
