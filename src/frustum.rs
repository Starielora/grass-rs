use ash::vk;

use crate::{
    overlay_drawable::OverlayDrawable,
    vkutils::{
        self, buffer::Buffer, push_constants::GPUPushConstantsTraditional, vk_destroy::VkDestroy,
    },
};

#[derive(Copy, Clone)]
#[repr(C)]
struct FrustumColorsGPU {
    planes_color: [f32; 4],
    edges_color: [f32; 4],
}

pub struct Frustum {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline_solid: vk::Pipeline,
    pipeline_wireframe: vk::Pipeline,
    descriptor_set: vk::DescriptorSet,
    color_buffer: Buffer,
    pub enabled: bool,
    pub planes_color: [f32; 4],
    pub edges_color: [f32; 4],
}

impl Frustum {
    pub fn new(ctx: &vkutils::context::VulkanContext) -> Frustum {
        let pipeline_layout = ctx.bindless_descriptor_set.traditional_pipeline_layout;
        let pipeline_solid = create_graphics_pipeline(
            &ctx.device,
            &ctx.swapchain.extent,
            &pipeline_layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            false,
            vk::FALSE,
        );
        let pipeline_wireframe = create_graphics_pipeline(
            &ctx.device,
            &ctx.swapchain.extent,
            &pipeline_layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            vk::PrimitiveTopology::LINE_LIST,
            true,
            vk::TRUE,
        );

        let color_buffer = ctx.create_bar_buffer(
            std::mem::size_of::<FrustumColorsGPU>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        Frustum {
            pipeline_layout,
            pipeline_solid,
            pipeline_wireframe,
            device: ctx.device.clone(),
            descriptor_set: ctx.bindless_descriptor_set.handle,
            color_buffer,
            enabled: true,
            planes_color: [1.0, 1.0, 1.0, 0.25],
            edges_color: [1.0, 1.0, 0.0, 1.0],
        }
    }
}

impl Drop for Frustum {
    fn drop(&mut self) {
        self.color_buffer.vk_destroy();

        unsafe {
            self.device.destroy_pipeline(self.pipeline_solid, None);
            self.device.destroy_pipeline(self.pipeline_wireframe, None);
        }
    }
}

impl OverlayDrawable for Frustum {
    fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        push_constants: &mut GPUPushConstantsTraditional,
    ) {
        self.color_buffer.update_contents(&[FrustumColorsGPU {
            planes_color: self.planes_color,
            edges_color: self.edges_color,
        }]);
        push_constants.frustum_colors = self.color_buffer.device_address.unwrap();

        unsafe {
            let sets = [self.descriptor_set];
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &sets,
                &[],
            );

            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstantsTraditional) as *const u8,
                    std::mem::size_of::<GPUPushConstantsTraditional>(),
                ),
            );

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_solid,
            );
            self.device.cmd_draw(command_buffer, 36, 1, 0, 0);

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_wireframe,
            );
            self.device.cmd_draw(command_buffer, 26, 1, 0, 0);
        }
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

fn create_graphics_pipeline(
    device: &ash::Device,
    window_extent: &vk::Extent2D,
    pipeline_layout: &vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
    topology: vk::PrimitiveTopology,
    is_wireframe: bool,
    depth_write: vk::Bool32,
) -> vk::Pipeline {
    // todo path lol
    let mut vs_spv_file = std::fs::File::open("target/debug/frustum.vert.spv").unwrap();
    let vs_spv = ash::util::read_spv(&mut vs_spv_file).unwrap();
    let vs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
    let vs_module = unsafe {
        device
            .create_shader_module(&vs_shader_module_create_info, None)
            .unwrap()
    };
    let shader_main = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let mut fs_spv_file = std::fs::File::open("target/debug/frustum.frag.spv").unwrap();
    let fs_spv = ash::util::read_spv(&mut fs_spv_file).unwrap();
    let fs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&fs_spv);
    let fs_module = unsafe {
        device
            .create_shader_module(&fs_shader_module_create_info, None)
            .unwrap()
    };

    let spec_entry = vk::SpecializationMapEntry {
        constant_id: 0,
        offset: 0,
        size: std::mem::size_of::<u32>(),
    };
    let spec_data: u32 = if is_wireframe { 1 } else { 0 };
    let spec_data_bytes = spec_data.to_ne_bytes();
    let spec_info = vk::SpecializationInfo {
        map_entry_count: 1,
        p_map_entries: &spec_entry,
        data_size: spec_data_bytes.len(),
        p_data: spec_data_bytes.as_ptr() as *const std::ffi::c_void,
        ..Default::default()
    };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vs_module,
            p_name: shader_main.as_ptr(),
            p_specialization_info: &spec_info,
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
        topology,
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
        front_face: vk::FrontFace::CLOCKWISE,
        depth_bias_enable: vk::FALSE,
        depth_bias_constant_factor: 1.25,
        depth_bias_clamp: 0.0,
        depth_bias_slope_factor: 1.75,
        line_width: if is_wireframe { 4.0 } else { 1.0 },
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
        depth_write_enable: depth_write,
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
