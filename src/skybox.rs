use crate::bindless_descriptor_set;
use crate::drawable;
use crate::gui_scene_node::GuiSceneNode;
use crate::push_constants::get_push_constants_range;
use crate::push_constants::GPUPushConstants;
use crate::vkutils;
use ash::vk;
use std::io::prelude::*;

pub struct Skybox {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    images: std::vec::Vec<vk::Image>,
    image_views: std::vec::Vec<vk::ImageView>,
    image_memories: std::vec::Vec<vk::DeviceMemory>,
    sampler: vk::Sampler,
    descriptor_set: vk::DescriptorSet,
    current_resource_id: u32,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    indices_count: usize,
    // gui_data: GuiData,
}

fn create_graphics_pipeline_layout(
    descriptor_set_layout: vk::DescriptorSetLayout,
    device: &ash::Device,
) -> vk::PipelineLayout {
    let set_layouts = [descriptor_set_layout];
    let push_constants_range = get_push_constants_range();
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);
    let pipeline_layout = unsafe { device.create_pipeline_layout(&create_info, None).unwrap() };

    pipeline_layout
}

fn create_graphics_pipeline(
    device: &ash::Device,
    window_extent: &vk::Extent2D,
    pipeline_layout: &vk::PipelineLayout,
    render_pass: &vk::RenderPass,
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

    let create_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(*pipeline_layout)
        .render_pass(*render_pass);

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

fn load_textures_to_staging_buffer(
    files: [&str; 6],
    vk: &vkutils::Context,
) -> (vk::Buffer, vk::DeviceMemory, u32, u32, isize) {
    let mut texture_width: i32 = 0;
    let mut texture_height: i32 = 0;

    let mut staging_buffer_offset: isize = 0;

    // these are initialized on first image
    let mut staging_buffer: Option<vk::Buffer> = None;
    let mut staging_buffer_memory: Option<vk::DeviceMemory> = None;
    let mut staging_buffer_ptr: Option<*mut std::ffi::c_void> = None;
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
            let (buffer, buffer_mem, _staging_buffer_allocated_size) = vk.create_buffer(
                total_size_in_bytes as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            );

            staging_buffer = Some(buffer);
            staging_buffer_memory = Some(buffer_mem);

            staging_buffer_ptr = Some(unsafe {
                vk.device
                    .map_memory(
                        staging_buffer_memory.unwrap(),
                        0,
                        vk::WHOLE_SIZE,
                        vk::MemoryMapFlags::empty(),
                    )
                    .expect("Failed to map memory for skybox image staging buffer")
            });
        } else if width != texture_width || height != texture_height {
            panic!(
                "Skybox images size mismatch. Expected {}x{}, got {}x{}",
                texture_width, texture_height, width, height
            );
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                img_data,
                staging_buffer_ptr.unwrap().offset(staging_buffer_offset) as *mut u8,
                single_texture_size_in_bytes.unwrap() as usize,
            );

            stb_image_rust::stbi_image_free(img_data);
        }

        staging_buffer_offset += single_texture_size_in_bytes.unwrap();
    }

    unsafe { vk.device.unmap_memory(staging_buffer_memory.unwrap()) };

    (
        staging_buffer.unwrap(),
        staging_buffer_memory.unwrap(),
        texture_width as u32,
        texture_height as u32,
        single_texture_size_in_bytes.unwrap(),
    )
}

fn load_textures(
    files: [&str; 6],
    vk: &vkutils::Context,
) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
    let (staging_buffer, staging_buffer_memory, width, height, single_image_size) =
        load_textures_to_staging_buffer(files, vk);

    let format = vk::Format::R8G8B8A8_UNORM;

    let image_create_info = vk::ImageCreateInfo::default()
        .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE)
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(6)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL) // TODO: change to optimal
        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    let image = unsafe {
        vk.device
            .create_image(&image_create_info, None)
            .expect("Failed to create skybox image")
    };

    let image_mem = vk.allocage_image_memory(image);

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .layer_count(6);

    let image_view_create_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::CUBE)
        .format(format)
        .components(vk::ComponentMapping::default())
        .subresource_range(subresource_range);

    let image_view = unsafe {
        vk.device
            .create_image_view(&image_view_create_info, None)
            .expect("Failed to create skybox image view")
    };

    // lastly copy data from staging buffer to image
    {
        let command_buffer = vk.create_command_buffer(vk::CommandBufferLevel::PRIMARY, true);
        vk.set_image_layout(
            command_buffer,
            image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            subresource_range,
            None,
            None,
        );

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
            vk.device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                buffer_copy_regions.as_slice(),
            )
        };

        vk.set_image_layout(
            command_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            subresource_range,
            None,
            None,
        );

        vk.flush_command_buffer(command_buffer, true);
    }

    // cleanup
    unsafe {
        vk.device.free_memory(staging_buffer_memory, None);
        vk.device.destroy_buffer(staging_buffer, None);
    }

    (image, image_mem, image_view)
}

impl Skybox {
    pub fn new(
        ctx: &vkutils::Context,
        cube_vertex_buffer: vk::Buffer,
        cube_index_buffer: vk::Buffer,
        indices_count: usize,
    ) -> Self {
        let pipeline_layout =
            create_graphics_pipeline_layout(ctx.descriptor_set_layout, &ctx.device);
        let pipeline = create_graphics_pipeline(
            &ctx.device,
            &ctx.window_extent,
            &pipeline_layout,
            &ctx.render_pass,
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

        let (image1, image_memory1, image_view1) = load_textures(skybox1_texture_files, &ctx);
        let (image2, image_memory2, image_view2) = load_textures(skybox2_texture_files, &ctx);

        let images = vec![image1, image2];
        let views = vec![image_view1, image_view2];
        let memories = vec![image_memory1, image_memory2];

        let mut descriptor_image_infos = std::vec::Vec::new();
        let mut descriptor_writes = std::vec::Vec::new();
        let mut skybox_resource_id = 0;

        for view in &views {
            let descriptor_image_info = [vk::DescriptorImageInfo::default()
                .sampler(sampler)
                .image_view(*view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

            descriptor_image_infos.push(descriptor_image_info);
        }

        for info in &descriptor_image_infos {
            descriptor_writes.push(
                vk::WriteDescriptorSet::default()
                    .dst_set(ctx.descriptor_set)
                    .dst_binding(bindless_descriptor_set::CUBE_SAMPLER_BINDING)
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

        Self {
            device: ctx.device.clone(),
            pipeline_layout,
            pipeline,
            images,
            image_views: views,
            image_memories: memories,
            sampler,
            descriptor_set: ctx.descriptor_set,
            current_resource_id: 0,
            vertex_buffer: cube_vertex_buffer,
            index_buffer: cube_index_buffer,
            indices_count,
        }
    }
}

impl std::ops::Drop for Skybox {
    fn drop(&mut self) {
        unsafe {
            for mem in &self.image_memories {
                self.device.free_memory(*mem, None);
            }
            for view in &self.image_views {
                self.device.destroy_image_view(*view, None);
            }
            for image in &self.images {
                self.device.destroy_image(*image, None);
            }
            self.device.destroy_sampler(self.sampler, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

impl drawable::Drawable for Skybox {
    fn cmd_draw(&mut self, command_buffer: &vk::CommandBuffer, push_constants: &GPUPushConstants) {
        unsafe {
            let sets = [self.descriptor_set];
            let dynamic_offsets = [];
            self.device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &sets,
                &dynamic_offsets,
            );

            self.device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            // FIXME quick hack to check if it works... these are only integers so there's no perf
            // penalty
            let mut pc = (*push_constants).clone();
            pc.current_skybox = self.current_resource_id;

            self.device.cmd_push_constants(
                *command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (&pc as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            let vertex_buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(*command_buffer, 0, &vertex_buffers, &offsets);
            self.device.cmd_bind_index_buffer(
                *command_buffer,
                self.index_buffer,
                0,
                vk::IndexType::UINT16,
            );
            self.device
                .cmd_draw_indexed(*command_buffer, self.indices_count as u32, 1, 0, 0, 0);
        }
    }
}

impl GuiSceneNode for Skybox {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        if ui.tree_node("Skybox").is_some() {
            ui.indent();
            if ui.selectable("daylight") {
                self.current_resource_id = 0;
            }
            if ui.selectable("learnopengl") {
                self.current_resource_id = 1;
            }
            ui.unindent();
        }
    }
}
