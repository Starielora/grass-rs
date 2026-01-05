use ash::vk;

use crate::{
    grid, mesh, skybox,
    vkutils::{
        self, descriptor_set::bindless, push_constants::GPUPushConstants, vk_destroy::VkDestroy,
    },
};

pub struct SceneColorPass {
    pub command_buffers: Vec<vk::CommandBuffer>,
    // TODO double buffering
    pub render_target: vkutils::image::Image,
    pub depth_image: vkutils::image::Image,

    pipeline: vk::Pipeline,
    device: ash::Device,
}

impl std::ops::Drop for SceneColorPass {
    fn drop(&mut self) {
        self.render_target.vk_destroy();
        self.depth_image.vk_destroy();
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl SceneColorPass {
    pub fn new(
        ctx: &mut vkutils::context::VulkanContext,
        skybox: &skybox::Skybox,
        grid: &grid::Grid,
        camera_data_buffer_address: vk::DeviceAddress,
        dir_light_data_buffer_address: vk::DeviceAddress,
        dir_light_camera_buffer_address: vk::DeviceAddress,
        shadow_map: (vk::Image, vk::ImageView),
        sampler: vk::Sampler,
        meshes: &[mesh::Mesh],
    ) -> Self {
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

        // TODO hardocded. This should be common with depth_map_display,
        let resource_id = 2;
        ctx.bindless_descriptor_set.update_sampler2d(
            shadow_map.1,
            sampler,
            vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL,
            resource_id,
        );

        for command_buffer in &command_buffers {
            record(
                &ctx.device,
                *command_buffer,
                &ctx.bindless_descriptor_set,
                (render_target.handle, render_target.view),
                (depth_image.handle, depth_image.view),
                (shadow_map.0, shadow_map.1),
                extent,
                &skybox,
                &grid,
                pipeline,
                pipeline_layout,
                camera_data_buffer_address,
                dir_light_data_buffer_address,
                dir_light_camera_buffer_address,
                resource_id,
                meshes,
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

fn record(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    descriptor_set: &bindless::DescriptorSet,
    color_image: (vk::Image, vk::ImageView),
    depth_image: (vk::Image, vk::ImageView),
    shadow_map_image: (vk::Image, vk::ImageView),
    extent: vk::Extent2D,
    skybox: &crate::skybox::Skybox,
    grid: &crate::grid::Grid,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    camera_buffer_address: vk::DeviceAddress,
    dir_light_buffer_address: vk::DeviceAddress,
    dir_light_camera_buffer_address: vk::DeviceAddress,
    depth_sampler_index: u32,
    meshes: &[mesh::Mesh],
) {
    let begin_info = vk::CommandBufferBeginInfo {
        ..Default::default()
    };
    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin command buffer");
    }

    record_image_barriers_for_scene_rendering(
        &device,
        command_buffer,
        color_image.0,
        depth_image.0,
        shadow_map_image.0,
    );

    begin_scene_rendering(
        &device,
        command_buffer,
        color_image.1,
        depth_image.1,
        extent,
    );

    descriptor_set.cmd_bind(command_buffer, vk::PipelineBindPoint::GRAPHICS);

    let mut push_constants = GPUPushConstants::default();
    push_constants.camera_data_buffer_address = camera_buffer_address;
    push_constants.dir_light_buffer_address = dir_light_buffer_address;
    push_constants.dir_light_camera_buffer_address = dir_light_camera_buffer_address;
    push_constants.depth_sampler_index = depth_sampler_index;

    skybox.record(command_buffer, &mut push_constants);

    unsafe {
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

    grid.record(command_buffer, &mut push_constants);

    unsafe {
        device.cmd_end_rendering(command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("Failed to end command buffer???");
    }
}

fn begin_scene_rendering(
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

fn record_image_barriers_for_scene_rendering(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    color_image: vk::Image,
    depth_image: vk::Image,
    shadow_map_image: vk::Image,
) {
    let color_subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(vk::REMAINING_ARRAY_LAYERS);

    vkutils::image_barrier(
        device,
        command_buffer,
        shadow_map_image,
        (
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        vkutils::depth_subresource_range(),
    );

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
    window_extent: &vk::Extent2D,
    pipeline_layout: vk::PipelineLayout,
    swapchain_format: vk::Format,
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

    let mut fs_spv_file = std::fs::File::open("target/debug/cube.frag.spv").unwrap();
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
