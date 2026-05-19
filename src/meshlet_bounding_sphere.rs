use ash::vk;

use crate::{
    assets::MeshletAsset,
    gui_scene_node::GuiSceneNode,
    overlay_drawable::OverlayDrawable,
    vkutils::{
        self,
        push_constants::{GPUPushConstantsMeshlet, GPUPushConstantsTraditional},
    },
};

pub struct MeshletBoundingSpheres {
    device: ash::Device,
    mesh_shader_device: ash::ext::mesh_shader::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: vk::DescriptorSet,
    // (meshlet_draws address, indirect buffer handle, draw count) per asset scene
    draw_infos: Vec<(vk::DeviceAddress, vk::Buffer, u32)>,
    enabled: bool,
}

impl MeshletBoundingSpheres {
    pub fn new(ctx: &vkutils::context::VulkanContext, assets: &[MeshletAsset]) -> Self {
        let pipeline_layout = ctx.bindless_descriptor_set.meshlet_pipeline_layout;
        let pipeline = create_pipeline(
            &ctx.device,
            &ctx.swapchain.extent,
            pipeline_layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
        );
        let draw_infos = assets
            .iter()
            .map(|a| a.scene_draw_info(a.default_scene.unwrap_or(0)))
            .collect();
        Self {
            device: ctx.device.clone(),
            mesh_shader_device: ctx.mesh_shader_device.clone(),
            pipeline,
            pipeline_layout,
            descriptor_set: ctx.bindless_descriptor_set.handle,
            draw_infos,
            enabled: false,
        }
    }
}

impl OverlayDrawable for MeshletBoundingSpheres {
    fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        push_constants: &mut GPUPushConstantsTraditional,
    ) {
        let sets = [self.descriptor_set];
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &sets,
                &[],
            );
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }

        let mut meshlet_pc = GPUPushConstantsMeshlet {
            camera: push_constants.camera,
            cull_camera: push_constants.cull_camera,
            meshlet_draws: 0,
            dir_light: 0,
            dir_light_camera: 0,
        };

        for (meshlet_draws_addr, indirect_buf, draws_count) in &self.draw_infos {
            meshlet_pc.meshlet_draws = *meshlet_draws_addr;
            unsafe {
                self.device.cmd_push_constants(
                    command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::TASK_EXT | vk::ShaderStageFlags::MESH_EXT,
                    0,
                    std::slice::from_raw_parts(
                        (&meshlet_pc as *const GPUPushConstantsMeshlet) as *const u8,
                        std::mem::size_of::<GPUPushConstantsMeshlet>(),
                    ),
                );
                self.mesh_shader_device.cmd_draw_mesh_tasks_indirect(
                    command_buffer,
                    *indirect_buf,
                    0,
                    *draws_count,
                    12,
                );
            }
        }
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

impl GuiSceneNode for MeshletBoundingSpheres {
    fn update(&mut self, ui: &imgui::Ui) {
        ui.checkbox("Meshlet bounding spheres", &mut self.enabled);
    }
}

impl Drop for MeshletBoundingSpheres {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

fn create_pipeline(
    device: &ash::Device,
    extent: &vk::Extent2D,
    pipeline_layout: vk::PipelineLayout,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
) -> vk::Pipeline {
    let shader_main = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let mut task_spv_file = std::fs::File::open("target/debug/meshlet.task.spv").unwrap();
    let task_spv = ash::util::read_spv(&mut task_spv_file).unwrap();
    let task_module = unsafe {
        device
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&task_spv), None)
            .unwrap()
    };

    let mut mesh_spv_file =
        std::fs::File::open("target/debug/meshlet_bounds_sphere.mesh.spv").unwrap();
    let mesh_spv = ash::util::read_spv(&mut mesh_spv_file).unwrap();
    let mesh_module = unsafe {
        device
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&mesh_spv), None)
            .unwrap()
    };

    let mut fs_spv_file =
        std::fs::File::open("target/debug/meshlet_bounds_sphere.frag.spv").unwrap();
    let fs_spv = ash::util::read_spv(&mut fs_spv_file).unwrap();
    let fs_module = unsafe {
        device
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&fs_spv), None)
            .unwrap()
    };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::TASK_EXT,
            module: task_module,
            p_name: shader_main.as_ptr(),
            ..Default::default()
        },
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
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        line_width: 1.5,
        ..Default::default()
    };

    let multisample_state = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_8,
        min_sample_shading: 1.0,
        ..Default::default()
    };

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: vk::TRUE,
        depth_write_enable: vk::FALSE,
        depth_compare_op: vk::CompareOp::GREATER,
        ..Default::default()
    };

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::TRUE,
        src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    };
    let attachments = [color_blend_attachment];
    let color_blend_state =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachments);

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
            .expect("Failed to create meshlet bounding sphere pipeline")
    };

    unsafe {
        device.destroy_shader_module(task_module, None);
        device.destroy_shader_module(mesh_module, None);
        device.destroy_shader_module(fs_module, None);
    }

    pipelines[0]
}
