use crate::gui_scene_node::GuiSceneNode;
use crate::vkutils;
use crate::vkutils::push_constants::GPUPushConstants;
use crate::vkutils::vk_destroy::VkDestroy;
use ash::vk;
use std::io::prelude::*;

pub struct Skybox {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    images: std::vec::Vec<vkutils::image::Image>,
    sampler: vk::Sampler,
    descriptor_set: vk::DescriptorSet,
    current_resource_id: u32,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    indices_count: usize,
    buffer: vkutils::buffer::Buffer,
    buffer_device_address: vk::DeviceAddress,
}

fn create_graphics_pipeline(
    device: &ash::Device,
    window_extent: &vk::Extent2D,
    pipeline_layout: &vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
) -> vk::Pipeline {
    // todo path lol
    let mut vs_spv_file = std::fs::File::open("target/debug/skybox.vert.spv").unwrap();
    let vs_spv = ash::util::read_spv(&mut vs_spv_file).unwrap();
    let vs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
    let vs_module = unsafe {
        device
            .create_shader_module(&vs_shader_module_create_info, None)
            .unwrap()
    };
    let shader_main = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let mut fs_spv_file = std::fs::File::open("target/debug/skybox.frag.spv").unwrap();
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
        front_face: vk::FrontFace::CLOCKWISE,
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

fn load_textures_to_staging_buffer2(
    files: [&str; 6],
    vk: &vkutils::context::VulkanContext,
) -> (vkutils::buffer::Buffer, u32, u32, isize) {
    let mut texture_width: i32 = 0;
    let mut texture_height: i32 = 0;

    let mut staging_buffer_offset: isize = 0;

    // these are initialized on first image
    let mut staging_buffer: Option<vkutils::buffer::Buffer> = None;
    let mut single_texture_size_in_bytes: Option<isize> = None;

    for path in files {
        let mut f = std::fs::File::open(path).expect("file not found");

        let mut contents = vec![];
        let mut width: i32 = 0;
        let mut height: i32 = 0;
        let mut comps: i32 = 0;
        let _ = f.read_to_end(&mut contents);

        let img_data = unsafe {
            stb_image_rust::stbi_load_from_memory(
                contents.as_ptr(),
                contents.len() as i32,
                &mut width,
                &mut height,
                &mut comps,
                stb_image_rust::STBI_rgb_alpha,
            )
        };

        // allocate staging buffer on first image
        if texture_width == 0 && texture_height == 0 {
            texture_width = width;
            texture_height = height;

            single_texture_size_in_bytes =
                Some(texture_width as isize * texture_height as isize * 4); // w * h * rgba comps
            let total_size_in_bytes = single_texture_size_in_bytes.unwrap() * 6; // 6 faces
            let buffer = vk.create_buffer(
                total_size_in_bytes as usize,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            );

            staging_buffer = Some(buffer);
        } else if width != texture_width || height != texture_height {
            panic!(
                "Skybox images size mismatch. Expected {}x{}, got {}x{}",
                texture_width, texture_height, width, height
            );
        }

        unsafe {
            // TODO fix this situation
            // maybe buffer should have a function to upload at offset
            std::ptr::copy_nonoverlapping(
                img_data,
                staging_buffer
                    .as_ref()
                    .unwrap()
                    .ptr
                    .unwrap()
                    .offset(staging_buffer_offset) as *mut u8,
                single_texture_size_in_bytes.unwrap() as usize,
            );

            stb_image_rust::stbi_image_free(img_data);
        }

        staging_buffer_offset += single_texture_size_in_bytes.unwrap();
    }

    staging_buffer.as_mut().unwrap().unmap_memory();

    (
        staging_buffer.unwrap(),
        texture_width as u32,
        texture_height as u32,
        single_texture_size_in_bytes.unwrap(),
    )
}

fn load_textures(files: [&str; 6], vk: &vkutils::context::VulkanContext) -> vkutils::image::Image {
    let (staging_buffer, width, height, single_image_size) =
        load_textures_to_staging_buffer2(files, vk);

    let format = vk::Format::R8G8B8A8_UNORM;

    let image = vkutils::image::Image::new(
        vk.device.clone(),
        vk::ImageCreateFlags::CUBE_COMPATIBLE,
        format,
        vk::Extent2D { width, height },
        6,
        vk::SampleCountFlags::TYPE_1,
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        vk::ImageAspectFlags::COLOR,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        &vk.physical_device.memory_props,
    );

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .layer_count(6);

    vk.transient_graphics_command_pool.transition_image_layout(
        vk.graphics_present_queue,
        image.handle,
        (
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        subresource_range,
    );

    vk.transient_transfer_command_pool
        .execute_short_lived_command_buffer(vk.transfer_queue, |device, command_buffer| {
            let mut buffer_copy_regions = Vec::new();

            for face in 0..6 as u64 {
                let image_subresource_layers = vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(face as u32)
                    .layer_count(1);
                let image_extent = vk::Extent3D::default()
                    .width(width as u32)
                    .height(height as u32)
                    .depth(1);
                let copy_region = vk::BufferImageCopy::default()
                    .image_subresource(image_subresource_layers)
                    .image_extent(image_extent)
                    .buffer_offset(face * single_image_size as u64);

                buffer_copy_regions.push(copy_region);
            }

            unsafe {
                device.cmd_copy_buffer_to_image(
                    command_buffer,
                    staging_buffer.handle,
                    image.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    buffer_copy_regions.as_slice(),
                )
            };
        });

    vk.transient_graphics_command_pool.transition_image_layout(
        vk.graphics_present_queue,
        image.handle,
        (
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        subresource_range,
    );

    // cleanup
    staging_buffer.vk_destroy();

    image
}

impl Skybox {
    pub fn new(
        ctx: &vkutils::context::VulkanContext,
        cube_vertex_buffer: vk::Buffer,
        cube_index_buffer: vk::Buffer,
        indices_count: usize,
    ) -> Self {
        let pipeline_layout = ctx.bindless_descriptor_set.pipeline_layout;
        let pipeline = create_graphics_pipeline(
            &ctx.device,
            &ctx.swapchain.extent,
            &pipeline_layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
        );

        let skybox1_texture_files = [
            "assets/skybox/daylight/Daylight Box_Right.png",
            "assets/skybox/daylight/Daylight Box_Left.png",
            "assets/skybox/daylight/Daylight Box_Top.png",
            "assets/skybox/daylight/Daylight Box_Bottom.png",
            "assets/skybox/daylight/Daylight Box_Front.png",
            "assets/skybox/daylight/Daylight Box_Back.png",
        ];

        let skybox2_texture_files = [
            "assets/skybox/learnopengl/right.png",
            "assets/skybox/learnopengl/left.png",
            "assets/skybox/learnopengl/top.png",
            "assets/skybox/learnopengl/bottom.png",
            "assets/skybox/learnopengl/front.png",
            "assets/skybox/learnopengl/back.png",
        ];

        let sampler_create_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .mip_lod_bias(0.0)
            .compare_op(vk::CompareOp::NEVER)
            .min_lod(0.0)
            .max_lod(1.0) // TODO mip levels
            .border_color(vk::BorderColor::INT_OPAQUE_WHITE)
            .max_anisotropy(1.0);

        let sampler = unsafe {
            ctx.device
                .create_sampler(&sampler_create_info, None)
                .expect("Failed to create image sampler")
        };

        let image1 = load_textures(skybox1_texture_files, ctx);
        let image2 = load_textures(skybox2_texture_files, ctx);

        let images = vec![image1, image2];

        let mut descriptor_image_infos = std::vec::Vec::new();
        let mut descriptor_writes = std::vec::Vec::new();
        let mut skybox_resource_id = 0;

        for image in &images {
            let descriptor_image_info = [vk::DescriptorImageInfo::default()
                .sampler(sampler)
                .image_view(image.view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

            descriptor_image_infos.push(descriptor_image_info);
        }

        for info in &descriptor_image_infos {
            descriptor_writes.push(
                vk::WriteDescriptorSet::default()
                    .dst_set(ctx.bindless_descriptor_set.handle)
                    .dst_binding(vkutils::descriptor_set::bindless::CUBE_SAMPLER_BINDING)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .dst_array_element(skybox_resource_id)
                    .image_info(info),
            );

            skybox_resource_id += 1;
        }

        let descriptor_copies = [];
        unsafe {
            ctx.device
                .update_descriptor_sets(&descriptor_writes, &descriptor_copies)
        };
        let buffer = ctx.create_bar_buffer(
            std::mem::size_of::<u32>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let buffer_device_address = buffer.device_address.unwrap();

        Self {
            device: ctx.device.clone(),
            pipeline_layout,
            pipeline,
            images,
            sampler,
            descriptor_set: ctx.bindless_descriptor_set.handle,
            current_resource_id: 0,
            vertex_buffer: cube_vertex_buffer,
            index_buffer: cube_index_buffer,
            indices_count,
            buffer,
            buffer_device_address,
        }
    }

    pub fn record(&self, command_buffer: vk::CommandBuffer, push_constants: &mut GPUPushConstants) {
        unsafe {
            let sets = [self.descriptor_set];
            let dynamic_offsets = [];
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &sets,
                &dynamic_offsets,
            );

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            push_constants.skybox_data = self.buffer_device_address;

            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            let vertex_buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer,
                0,
                vk::IndexType::UINT16,
            );
            self.device
                .cmd_draw_indexed(command_buffer, self.indices_count as u32, 1, 0, 0, 0);
        }
    }

    fn refresh_per_frame_buffer(&self) {
        self.buffer.update_contents(&[self.current_resource_id]);
    }
}

impl std::ops::Drop for Skybox {
    fn drop(&mut self) {
        self.buffer.vk_destroy();
        for image in &self.images {
            image.vk_destroy();
        }
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl GuiSceneNode for Skybox {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        let mut refresh = false;
        if ui
            .tree_node_config("Skybox")
            .opened(true, imgui::Condition::Appearing)
            .push()
            .is_some()
        {
            ui.indent();
            if ui.selectable("daylight") {
                self.current_resource_id = 0;
                refresh = true;
            }
            if ui.selectable("learnopengl") {
                self.current_resource_id = 1;
                refresh = true;
            }
            ui.unindent();
        }

        if refresh {
            self.refresh_per_frame_buffer();
        }
    }
}
