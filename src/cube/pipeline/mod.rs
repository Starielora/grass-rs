use crate::push_constants::get_push_constants_range;
use crate::vkutils;
use ash::vk;

pub struct Pipeline {
    device: ash::Device,
    pub(in crate::cube) pipeline_layout: vk::PipelineLayout,
    pub(in crate::cube) pipeline: vk::Pipeline,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    pub(in crate::cube) vertex_buffer_device_address: vk::DeviceAddress,
}

fn create_graphics_pipeline_layout(device: &ash::Device) -> vk::PipelineLayout {
    let layouts = [];

    let push_constants_range = get_push_constants_range();
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&layouts)
        .push_constant_ranges(&push_constants_range);
    unsafe { device.create_pipeline_layout(&create_info, None).unwrap() }
}

fn create_graphics_pipeline(
    device: &ash::Device,
    window_extent: &vk::Extent2D,
    pipeline_layout: &vk::PipelineLayout,
    render_pass: &vk::RenderPass,
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

impl Pipeline {
    pub fn new(ctx: &vkutils::Context) -> Self {
        const VERTEX_DATA_SIZE: usize = 8 * 6 * 6;
        #[rustfmt::skip]
        static VERTICES: [f32; VERTEX_DATA_SIZE] = [
            // positions       // normals        // texture coords
            -0.5, -0.5, -0.5,  0.0,  0.0, -1.0,  0.0,  0.0,
             0.5, -0.5, -0.5,  0.0,  0.0, -1.0,  1.0,  0.0,
             0.5,  0.5, -0.5,  0.0,  0.0, -1.0,  1.0,  1.0,
             0.5,  0.5, -0.5,  0.0,  0.0, -1.0,  1.0,  1.0,
            -0.5,  0.5, -0.5,  0.0,  0.0, -1.0,  0.0,  1.0,
            -0.5, -0.5, -0.5,  0.0,  0.0, -1.0,  0.0,  0.0,

             0.5, -0.5,  0.5,  0.0,  0.0,  1.0,  1.0,  0.0,
            -0.5, -0.5,  0.5,  0.0,  0.0,  1.0,  0.0,  0.0,
             0.5,  0.5,  0.5,  0.0,  0.0,  1.0,  1.0,  1.0,
            -0.5,  0.5,  0.5,  0.0,  0.0,  1.0,  0.0,  1.0,
             0.5,  0.5,  0.5,  0.0,  0.0,  1.0,  1.0,  1.0,
            -0.5, -0.5,  0.5,  0.0,  0.0,  1.0,  0.0,  0.0,

            -0.5,  0.5, -0.5, -1.0,  0.0,  0.0,  1.0,  1.0,
            -0.5,  0.5,  0.5, -1.0,  0.0,  0.0,  1.0,  0.0,
            -0.5, -0.5, -0.5, -1.0,  0.0,  0.0,  0.0,  1.0,
            -0.5, -0.5,  0.5, -1.0,  0.0,  0.0,  0.0,  0.0,
            -0.5, -0.5, -0.5, -1.0,  0.0,  0.0,  0.0,  1.0,
            -0.5,  0.5,  0.5, -1.0,  0.0,  0.0,  1.0,  0.0,

             0.5,  0.5,  0.5,  1.0,  0.0,  0.0,  1.0,  0.0,
             0.5,  0.5, -0.5,  1.0,  0.0,  0.0,  1.0,  1.0,
             0.5, -0.5, -0.5,  1.0,  0.0,  0.0,  0.0,  1.0,
             0.5, -0.5, -0.5,  1.0,  0.0,  0.0,  0.0,  1.0,
             0.5, -0.5,  0.5,  1.0,  0.0,  0.0,  0.0,  0.0,
             0.5,  0.5,  0.5,  1.0,  0.0,  0.0,  1.0,  0.0,

             0.5, -0.5, -0.5,  0.0, -1.0,  0.0,  1.0,  1.0,
            -0.5, -0.5, -0.5,  0.0, -1.0,  0.0,  0.0,  1.0,
             0.5, -0.5,  0.5,  0.0, -1.0,  0.0,  1.0,  0.0,
            -0.5, -0.5,  0.5,  0.0, -1.0,  0.0,  0.0,  0.0,
             0.5, -0.5,  0.5,  0.0, -1.0,  0.0,  1.0,  0.0,
            -0.5, -0.5, -0.5,  0.0, -1.0,  0.0,  0.0,  1.0,

            -0.5,  0.5, -0.5,  0.0,  1.0,  0.0,  0.0,  1.0,
             0.5,  0.5, -0.5,  0.0,  1.0,  0.0,  1.0,  1.0,
             0.5,  0.5,  0.5,  0.0,  1.0,  0.0,  1.0,  0.0,
             0.5,  0.5,  0.5,  0.0,  1.0,  0.0,  1.0,  0.0,
            -0.5,  0.5,  0.5,  0.0,  1.0,  0.0,  0.0,  0.0,
            -0.5,  0.5, -0.5,  0.0,  1.0,  0.0,  0.0,  1.0
        ];

        let pipeline_layout = create_graphics_pipeline_layout(&ctx.device);
        let pipeline = create_graphics_pipeline(
            &ctx.device,
            &ctx.window_extent,
            &pipeline_layout,
            &ctx.render_pass,
        );
        let (vertex_buffer, vertex_buffer_memory, allocation_size) = ctx.create_buffer(
            (VERTEX_DATA_SIZE * std::mem::size_of::<f32>()) as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let vertex_buffer_ptr = unsafe {
            ctx.device
                .map_memory(
                    vertex_buffer_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Could not map cube buffer memory")
        };

        unsafe {
            ash::util::Align::new(
                vertex_buffer_ptr,
                std::mem::align_of::<[f32; VERTEX_DATA_SIZE]>() as u64,
                allocation_size,
            )
            .copy_from_slice(&VERTICES);
        };

        let vertex_buffer_device_address = unsafe {
            let address_info = vk::BufferDeviceAddressInfo {
                buffer: vertex_buffer,
                ..Default::default()
            };
            ctx.device.get_buffer_device_address(&address_info)
        };

        Self {
            device: ctx.device.clone(),
            pipeline_layout,
            pipeline,
            vertex_buffer,
            vertex_buffer_memory,
            vertex_buffer_device_address,
        }
    }
}

impl std::ops::Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}
