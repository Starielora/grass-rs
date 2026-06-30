use ash::vk;

use crate::vkutils::{self, shaders, vk_destroy::VkDestroy};

pub struct Skybox2 {
    vk: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: vk::DescriptorSet,
    push_constants: PushConstants,
    images: std::vec::Vec<vkutils::image::Image>,
    sampler: vkutils::sampler::Sampler,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct PushConstants {
    view_camera: vk::DeviceAddress,
    current_texture_id: u32,
}

impl std::ops::Drop for Skybox2 {
    fn drop(&mut self) {
        unsafe {
            self.vk.destroy_pipeline_layout(self.pipeline_layout, None);
            self.vk.destroy_pipeline(self.pipeline, None);
            for image in &self.images {
                image.vk_destroy();
            }
            self.sampler.vk_destroy();
        }
    }
}

impl Skybox2 {
    pub fn new(
        ctx: &vkutils::context::VulkanContext,
        view_camera: vk::DeviceAddress,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let vk = &ctx.device.clone();
        let surface_format = ctx.swapchain.surface_format.format;
        let depth_format = ctx.depth_format;
        let descriptor_set_layout = ctx.bindless_descriptor_set.layout;

        let pipeline_layout = create_pipeline_layout(vk, descriptor_set_layout);
        let pipeline = create_graphics_pipeline(vk, surface_format, depth_format, pipeline_layout)?;

        let texture1 = ctx.load_cubemap_texture(SKYBOX1_TEXTURES);
        let texture2 = ctx.load_cubemap_texture(SKYBOX2_TEXTURES);

        let sampler = ctx.create_sampler();
        let textures = vec![texture1, texture2];

        let mut descriptor_image_infos = std::vec::Vec::new();
        let mut descriptor_writes = std::vec::Vec::new();

        for image in &textures {
            let descriptor_image_info = [vk::DescriptorImageInfo::default()
                .sampler(sampler.handle)
                .image_view(image.view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

            descriptor_image_infos.push(descriptor_image_info);
        }

        for (i, info) in descriptor_image_infos.iter().enumerate() {
            descriptor_writes.push(
                vk::WriteDescriptorSet::default()
                    .dst_set(ctx.bindless_descriptor_set.handle)
                    .dst_binding(vkutils::descriptor_set::bindless::CUBE_SAMPLER_BINDING)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .dst_array_element(i as u32)
                    .image_info(info),
            );
        }

        let descriptor_copies = [];
        unsafe {
            ctx.device
                .update_descriptor_sets(&descriptor_writes, &descriptor_copies)
        };

        let push_constants = PushConstants {
            view_camera,
            current_texture_id: 0,
        };

        Ok(Self {
            vk: vk.clone(),
            pipeline,
            pipeline_layout,
            descriptor_set: ctx.bindless_descriptor_set.handle,
            push_constants,
            images: textures,
            sampler,
        })
    }

    pub fn record(&self, command_buffer: vk::CommandBuffer, extent: vk::Extent2D) {
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

            vk.cmd_set_viewport(command_buffer, 0, &[viewport]);
            vk.cmd_set_scissor(command_buffer, 0, &[scissors]);

            let sets = [self.descriptor_set];
            let dynamic_offsets = [];
            vk.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &sets,
                &dynamic_offsets,
            );

            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                get_push_constants_stage_flags(),
                0,
                self.push_constants_data(),
            );

            vk.cmd_draw(command_buffer, 36, 1, 0, 0);
        }
    }

    fn push_constants_data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (&self.push_constants as *const PushConstants) as *const u8,
                std::mem::size_of::<PushConstants>(),
            )
        }
    }
}

fn create_graphics_pipeline(
    vk: &ash::Device,
    surface_format: vk::Format,
    depth_format: vk::Format,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, Box<dyn std::error::Error>> {
    let vs = &shaders::SKYBOX_VERT;
    let fs = &shaders::SKYBOX_FRAG;

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

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();

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

    Ok(pipelines[0])
}

fn get_push_constants_stage_flags() -> vk::ShaderStageFlags {
    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
}

fn get_push_constant_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: get_push_constants_stage_flags(),
        offset: 0,
        size: std::mem::size_of::<PushConstants>() as u32,
    }]
}

fn create_pipeline_layout(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
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

macro_rules! embed_textures {
    ([ $($path:literal),* $(,)? ]) => {
            [
                $(
                    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
                ),*
            ]
    };
}

static SKYBOX1_TEXTURES: [&'static [u8]; 6] = embed_textures!([
    "assets/skybox/daylight/Daylight Box_Right.png",
    "assets/skybox/daylight/Daylight Box_Left.png",
    "assets/skybox/daylight/Daylight Box_Top.png",
    "assets/skybox/daylight/Daylight Box_Bottom.png",
    "assets/skybox/daylight/Daylight Box_Front.png",
    "assets/skybox/daylight/Daylight Box_Back.png",
]);

static SKYBOX2_TEXTURES: [&'static [u8]; 6] = embed_textures!([
    "assets/skybox/learnopengl/right.png",
    "assets/skybox/learnopengl/left.png",
    "assets/skybox/learnopengl/top.png",
    "assets/skybox/learnopengl/bottom.png",
    "assets/skybox/learnopengl/front.png",
    "assets/skybox/learnopengl/back.png",
]);
