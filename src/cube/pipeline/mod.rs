use crate::gltf_loader;
use crate::push_constants::get_push_constants_range;
use crate::vkutils;
use ash::vk::{self, VertexInputRate};

pub struct Pipeline {
    device: ash::Device,
    pub(in crate::cube) pipeline_layout: vk::PipelineLayout,
    pub(in crate::cube) pipeline: vk::Pipeline,
    pub vertex_buffer: vk::Buffer, // TODO this would be better to be mod private. I need to move skybox
    vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    pub indices_count: usize,
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

    let vertex_binding_desciptions = [vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride((std::mem::size_of::<f32>() * 8) as u32)
        .input_rate(VertexInputRate::VERTEX)];

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

fn upload_vertex_buffer(
    vertex_data: &Vec<f32>,
    ctx: &vkutils::Context,
) -> (vk::Buffer, vk::DeviceMemory) {
    let (vertex_buffer, vertex_buffer_memory, allocation_size) = ctx.create_buffer(
        (vertex_data.len() * std::mem::size_of::<f32>()) as u64,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    unsafe {
        let vertex_buffer_ptr = ctx
            .device
            .map_memory(
                vertex_buffer_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Could not map cube buffer memory");

        ash::util::Align::new(
            vertex_buffer_ptr,
            std::mem::align_of::<f32>() as u64,
            allocation_size,
        )
        .copy_from_slice(&vertex_data.as_slice());

        ctx.device.unmap_memory(vertex_buffer_memory);
    };

    (vertex_buffer, vertex_buffer_memory)
}

// TODO this is the same as before, just different type.
fn upload_index_buffer(
    index_data: &Vec<u16>,
    ctx: &vkutils::Context,
) -> (vk::Buffer, vk::DeviceMemory) {
    let (vertex_buffer, vertex_buffer_memory, allocation_size) = ctx.create_buffer(
        (index_data.len() * std::mem::size_of::<u16>()) as u64,
        vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    unsafe {
        let vertex_buffer_ptr = ctx
            .device
            .map_memory(
                vertex_buffer_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Could not map cube buffer memory");

        ash::util::Align::new(
            vertex_buffer_ptr,
            std::mem::align_of::<u16>() as u64,
            allocation_size,
        )
        .copy_from_slice(&index_data.as_slice());

        ctx.device.unmap_memory(vertex_buffer_memory);
    };

    (vertex_buffer, vertex_buffer_memory)
}

impl Pipeline {
    pub fn new(ctx: &vkutils::Context) -> Self {
        let (vertex_data, index_data) =
            gltf_loader::load("C:/Users/edyko/dev/grass-rs/assets/cube.gltf");

        let pipeline_layout = create_graphics_pipeline_layout(&ctx.device);
        let pipeline = create_graphics_pipeline(
            &ctx.device,
            &ctx.window_extent,
            &pipeline_layout,
            &ctx.render_pass,
        );

        let (vertex_buffer, vertex_buffer_memory) = upload_vertex_buffer(&vertex_data, &ctx);
        let (index_buffer, index_buffer_memory) = upload_index_buffer(&index_data, &ctx);

        Self {
            device: ctx.device.clone(),
            pipeline_layout,
            pipeline,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            indices_count: index_data.len(),
        }
    }
}

impl std::ops::Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}
