use crate::assets::gltf_asset;
use crate::camera::GPUCameraData;
use crate::frustum2::Frustum2;
use crate::grid2::Grid2;
use crate::gui2;
use crate::meshlet2::{self};
use crate::skybox2::Skybox2;
use crate::vkutils::gpu_profiler::GpuProfiler;
use crate::vkutils::{self, vk_destroy::VkDestroy};
use ash::vk;
use glm;
use rand::RngExt;
use vkutils::FRAMES_IN_FLIGHT;

struct PerFrameData {
    vk: ash::Device,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    acquire_semaphore: vk::Semaphore,
    view_camera: vkutils::buffer::Buffer,
    cull_camera: vkutils::buffer::Buffer,
    render_target: vkutils::image::Image,
    depth_image: vkutils::image::Image,
    visible_count: vkutils::buffer::Buffer,
    visible_instances: vkutils::buffer::Buffer,
}

impl VkDestroy for PerFrameData {
    fn vk_destroy(&self) {
        let vk = &self.vk;
        unsafe {
            vk.destroy_fence(self.fence, None);
            vk.destroy_semaphore(self.acquire_semaphore, None);
        }
        self.view_camera.vk_destroy();
        self.cull_camera.vk_destroy();
        self.render_target.vk_destroy();
        self.depth_image.vk_destroy();
        self.visible_count.vk_destroy();
        self.visible_instances.vk_destroy();
    }
}

pub struct Renderer2 {
    vk: ash::Device,

    grid: Grid2,
    skybox: Skybox2,
    frustum: Frustum2,
    frustum_enabled: bool,
    bounding_sphere: meshlet2::bounding_sphere::BoundingSphere,

    compute_visible_meshlets_pipeline: meshlet2::compute::compute_visible_meshlets::Pipeline,
    prep_draw_mesh_tasks_command_pipeline:
        meshlet2::compute::prep_draw_mesh_tasks_command::Pipeline,
    meshlet_pipeline: meshlet2::GraphicsPipeline,
    geometry_data: meshlet2::GeometryBuffers,
    draw_params_buf: vkutils::buffer::Buffer,

    frames: [PerFrameData; vkutils::FRAMES_IN_FLIGHT],
    finished_semaphores: std::vec::Vec<vk::Semaphore>,
}

impl std::ops::Drop for Renderer2 {
    fn drop(&mut self) {
        self.draw_params_buf.vk_destroy();
        self.geometry_data.vk_destroy();
        for f in &self.frames {
            f.vk_destroy();
        }
        for s in &self.finished_semaphores {
            unsafe {
                self.vk.destroy_semaphore(*s, None);
            }
        }
    }
}

pub enum FrameOutcome {
    Presented,
    RebuildSwapchain,
}

impl Renderer2 {
    pub fn new(ctx: &mut vkutils::context::VulkanContext) -> Renderer2 {
        let command_buffers = ctx.graphics_command_pool.allocate_command_buffers(
            vk::CommandBufferLevel::PRIMARY,
            FRAMES_IN_FLIGHT.try_into().unwrap(),
        );

        let grid = Grid2::new(
            &ctx.device,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            ctx.bindless_descriptor_set.layout,
        )
        .expect("Failed to instantiate Grid object");

        let skybox = Skybox2::new(&ctx).expect("Failed to instantiate skybox");

        let frustum = Frustum2::new(
            &ctx.device,
            ctx.bindless_descriptor_set.layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
        );

        let brabon_data = gltf_asset::GltfAssetData::new(
            "/home/starielora/dev/repos/Vulkan-Assets/models/chinesedragon.gltf",
        );

        let mut geometry_builder = meshlet2::gltf::GeometryBuilder::new();
        geometry_builder.add_instance(&brabon_data, glm::Mat4::identity(), Option::None);

        {
            let mut rng = rand::rng();

            for _i in 0..200 {
                let tx: f32 = rng.random_range(-10.0f32..10.0f32);
                let ty: f32 = rng.random_range(-10.0f32..10.0f32);
                let tz: f32 = rng.random_range(-10.0f32..10.0f32);

                let az: f32 = rng.random_range(0.0f32..360.0f32).to_radians();
                let el: f32 = rng.random_range(-90.0f32..90.0f32).to_radians();

                let mut mat = glm::Mat4::identity();

                mat = glm::translate(&mat, &glm::make_vec3(&[tx, ty, tz]));
                mat = glm::rotate(&mat, az, &glm::make_vec3(&[0.0, -1.0, 0.0]));
                mat = glm::rotate(&mat, el, &glm::make_vec3(&[0.0, 0.0, 1.0]));

                geometry_builder.add_instance(&brabon_data, mat, Option::None);
            }
        }

        let mut mat = glm::Mat4::identity();
        mat = glm::translate(&mat, &glm::make_vec3(&[1.0, 1.0, 1.0]));
        mat = glm::rotate(&mat, 45.0f32.to_radians(), &glm::make_vec3(&[1.0, 1.0, 0.]));
        geometry_builder.add_instance(&brabon_data, mat, Option::None);

        println!(
            "Total meshlet instances: {}",
            geometry_builder.geometry_data.meshlet_instances.len()
        );

        // let bistro_data = gltf_asset::GltfAssetData::new(
        //     "/home/starielora/dev/repos/RTXDI-Assets/bistro/bistro.gltf",
        // );
        // geometry_builder.add_instance(&bistro_data, glm::Mat4::identity(), Option::None);
        // geometry_builder.add_instance(&bistro_data, mat, Option::None);

        let geometry_data = meshlet2::GeometryBuffers::new(&ctx, &geometry_builder.geometry_data);
        let subgroup_size = ctx.physical_device.subgroup_size;
        let draw_mesh_tasks_command_buf = create_draw_mesh_tasks_command_buf(&ctx);
        let meshlet_pipeline = meshlet2::GraphicsPipeline::new(
            &ctx.device,
            &ctx.mesh_shader_device,
            ctx.bindless_descriptor_set.layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            draw_mesh_tasks_command_buf.handle,
            subgroup_size,
        );

        let bounding_sphere = meshlet2::bounding_sphere::BoundingSphere::new(
            &ctx.device,
            &ctx.mesh_shader_device,
            ctx.bindless_descriptor_set.layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            ctx.physical_device.subgroup_size,
            ctx.physical_device.max_task_workgroup_count,
            draw_mesh_tasks_command_buf.handle,
        );

        let compute_visible_meshlets_pipeline =
            meshlet2::compute::compute_visible_meshlets::Pipeline::new(
                &ctx.device,
                ctx.bindless_descriptor_set.layout,
                ctx.physical_device.subgroup_size,
            );

        let prep_draw_mesh_tasks_command_pipeline =
            meshlet2::compute::prep_draw_mesh_tasks_command::Pipeline::new(
                &ctx.device,
                ctx.bindless_descriptor_set.layout,
                ctx.physical_device.subgroup_size,
            );

        let finished_semaphores = (0..ctx.swapchain.images.len())
            .map(|_| ctx.create_semaphore_vk())
            .collect();

        let mut command_buffer_id = 0;
        let per_frame_data = std::array::from_fn(|_| {
            let command_buffer = command_buffers[command_buffer_id];
            command_buffer_id += 1;
            let view_camera = ctx.create_bar_buffer(
                size_of::<GPUCameraData>(),
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let cull_camera = ctx.create_bar_buffer(
                size_of::<GPUCameraData>(),
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

            let fence = ctx.create_fence_vk(true);
            let acquire_semaphore = ctx.create_semaphore_vk();

            let render_target = create_render_target_image(&ctx);
            let depth_image = create_depth_image(&ctx);

            // stub - is overwritten in compute prepass
            let mut visible_meshlets_instances: std::vec::Vec<meshlet2::gpu::MeshletInstance> =
                vec![];
            visible_meshlets_instances.resize(
                geometry_data.meshlet_instances_count as usize,
                meshlet2::gpu::MeshletInstance {
                    mesh_instance_index: 0,
                    meshlet_index: 0,
                    lod_index: 0,
                },
            );
            let visible_meshlets_instances_buffer = ctx.upload_buffer(
                &visible_meshlets_instances,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

            let visible_meshlets_instances_count_buffer = ctx.upload_buffer(
                &vec![0 as u32],
                vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::TRANSFER_DST,
            );

            let frame = PerFrameData {
                vk: ctx.device.clone(),
                command_buffer,
                fence,
                acquire_semaphore,
                view_camera,
                cull_camera,
                render_target,
                depth_image,
                visible_count: visible_meshlets_instances_count_buffer,
                visible_instances: visible_meshlets_instances_buffer,
            };

            frame
        });

        Self {
            vk: ctx.device.clone(),
            grid,
            skybox,
            frustum,
            frustum_enabled: true,
            bounding_sphere,
            meshlet_pipeline,
            geometry_data,
            draw_params_buf: draw_mesh_tasks_command_buf,
            compute_visible_meshlets_pipeline,
            prep_draw_mesh_tasks_command_pipeline,
            frames: per_frame_data,
            finished_semaphores,
        }
    }

    pub fn toggle_frustum(&mut self) {
        self.frustum_enabled = !self.frustum_enabled;
    }

    pub fn toggle_bounding_sphere_mode(&mut self) {
        self.bounding_sphere.toggle_mode();
    }

    // TODO make type safe - don't rely on tuple indices - easy to mix
    pub fn update_gpu_camera_data(
        &self,
        view_camera_data: (glm::Vec4, glm::Mat4, glm::Mat4),
        cull_camera_data: (glm::Vec4, glm::Mat4, glm::Mat4),
        frame_in_flight: usize,
    ) {
        self.frames[frame_in_flight]
            .view_camera
            .update_contents(&[GPUCameraData {
                pos: view_camera_data.0,
                projview: view_camera_data.1,
                view: view_camera_data.2,
            }]);
        self.frames[frame_in_flight]
            .cull_camera
            .update_contents(&[GPUCameraData {
                pos: cull_camera_data.0,
                projview: cull_camera_data.1,
                view: cull_camera_data.2,
            }]);
    }

    pub fn resize(&mut self, ctx: &vkutils::context::VulkanContext) {
        for frame in &mut self.frames {
            frame.render_target.vk_destroy();
            frame.depth_image.vk_destroy();

            frame.render_target = create_render_target_image(&ctx);
            frame.depth_image = create_depth_image(&ctx);
        }
    }

    // TODO unsure if must be outside
    pub fn wait_fences(&self, frame_in_flight: usize) {
        // TODO protect against acquire returning the same image twice in a row
        unsafe {
            self.vk
                .wait_for_fences(&[self.frames[frame_in_flight].fence], true, !0)
                .expect("Failed wait for fences");
        }
    }

    pub fn draw(
        &self,
        vkctx: &mut vkutils::context::VulkanContext,
        gui: &mut gui2::Gui2,
        profiler: &mut GpuProfiler,
        frame_in_flight: usize,
    ) -> FrameOutcome {
        let vk = &self.vk;
        let frame = &self.frames[frame_in_flight];

        let acq_sem = frame.acquire_semaphore;
        let fence = frame.fence;

        let acquire_result = vkctx
            .swapchain
            .acquire_next_image2(!0, acq_sem, vk::Fence::null());

        let (image_index, is_swapchain_suboptimal) = match acquire_result {
            Ok(v) => v,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return FrameOutcome::RebuildSwapchain,
            Err(e) => panic!("{e:?}"),
        };

        let image_index: usize = image_index.try_into().unwrap();

        let fin_sem = self.finished_semaphores[image_index];

        let queue = vkctx.graphics_present_queue;
        let command_buffer = frame.command_buffer;

        let view_camera_bda = frame.view_camera.device_address.unwrap();
        let cull_camera_bda = frame.cull_camera.device_address.unwrap();
        let visible_meshlet_instances_bda = frame.visible_instances.device_address.unwrap();
        let visible_meshlet_instances_count_bda = frame.visible_count.device_address.unwrap();

        unsafe {
            vk.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("Failed to reset command buffer");

            let begin_info = vk::CommandBufferBeginInfo {
                ..Default::default()
            };
            vk.begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");

            profiler.begin_frame(command_buffer, frame_in_flight);
            let gpu_total_scope = profiler.begin(command_buffer, "gpu_total");

            {
                vk.cmd_fill_buffer(
                    command_buffer,
                    frame.visible_count.handle,
                    0,
                    std::mem::size_of::<u32>() as u64,
                    0,
                );

                let scope = profiler.begin(command_buffer, "compute_visible_meshlets");
                self.compute_visible_meshlets_pipeline.record(
                    command_buffer,
                    &self.geometry_data,
                    cull_camera_bda,
                    visible_meshlet_instances_bda,
                    visible_meshlet_instances_count_bda,
                );
                profiler.end(command_buffer, scope);

                let barrier = vk::MemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE);

                vk.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::COMPUTE_SHADER, // src: compute prepass
                    vk::PipelineStageFlags::COMPUTE_SHADER,
                    vk::DependencyFlags::empty(),
                    &[barrier],
                    &[],
                    &[],
                );

                let scope = profiler.begin(command_buffer, "prep_draw_mesh_tasks_command");
                self.prep_draw_mesh_tasks_command_pipeline.record(
                    command_buffer,
                    self.draw_params_buf.device_address.unwrap(),
                    visible_meshlet_instances_count_bda,
                );
                profiler.end(command_buffer, scope);

                let barrier = vk::MemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                    .dst_access_mask(
                        vk::AccessFlags::INDIRECT_COMMAND_READ | vk::AccessFlags::SHADER_READ,
                    );

                vk.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::COMPUTE_SHADER,
                    vk::PipelineStageFlags::DRAW_INDIRECT
                        | vk::PipelineStageFlags::TASK_SHADER_EXT
                        | vk::PipelineStageFlags::MESH_SHADER_EXT,
                    vk::DependencyFlags::empty(),
                    &[barrier],
                    &[],
                    &[],
                );
            }

            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [153.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0, 1.0],
                },
            };

            let depth_clear_value = vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 0.0,
                    stencil: 0,
                },
            };

            let render_target = &frame.render_target;
            let depth_image = &frame.depth_image;

            let (color_image, color_image_view) = (render_target.handle, render_target.view);
            let (depth_image, depth_image_view) = (depth_image.handle, depth_image.view);

            let color_subresource_range = vkutils::color_subresource_range();

            {
                vkutils::image_barrier(
                    vk,
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
                    vk,
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

            {
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
                        extent: vkctx.swapchain.extent,
                        offset: vk::Offset2D { x: 0, y: 0 },
                    })
                    .layer_count(1)
                    .color_attachments(&color_attachments)
                    .depth_attachment(&depth_attachment);

                vk.cmd_begin_rendering(command_buffer, &rendering_info);
            }

            let scope = profiler.begin(command_buffer, "draw_mesh_tasks");
            self.meshlet_pipeline.record(
                command_buffer,
                vkctx.swapchain.extent,
                &self.geometry_data,
                view_camera_bda,
                cull_camera_bda,
                visible_meshlet_instances_bda,
                visible_meshlet_instances_count_bda,
            );
            profiler.end(command_buffer, scope);

            self.skybox
                .record(command_buffer, vkctx.swapchain.extent, view_camera_bda);

            self.bounding_sphere.record(
                command_buffer,
                vkctx.swapchain.extent,
                &self.geometry_data,
                view_camera_bda,
                visible_meshlet_instances_bda,
                visible_meshlet_instances_count_bda,
            );

            if self.frustum_enabled {
                self.frustum.record(
                    command_buffer,
                    vkctx.swapchain.extent,
                    view_camera_bda,
                    cull_camera_bda,
                );
            }
            self.grid
                .record(command_buffer, vkctx.swapchain.extent, view_camera_bda);

            gui.record(command_buffer);

            vk.cmd_end_rendering(command_buffer);

            {
                let present_image = vkctx.swapchain.images[image_index];

                vkutils::image_barrier(
                    vk,
                    command_buffer,
                    color_image,
                    (
                        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    ),
                    (
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        vk::AccessFlags::TRANSFER_READ,
                        vk::PipelineStageFlags::TRANSFER,
                    ),
                    color_subresource_range,
                );

                vkutils::image_barrier(
                    vk,
                    command_buffer,
                    present_image,
                    (
                        vk::ImageLayout::UNDEFINED,
                        vk::AccessFlags::NONE,
                        vk::PipelineStageFlags::TRANSFER,
                    ),
                    (
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::PipelineStageFlags::TRANSFER,
                    ),
                    color_subresource_range,
                );

                // MSAA resolve
                let scope = profiler.begin(command_buffer, "msaa_resolve");
                let subresource = vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1);

                let resolve_region = vk::ImageResolve::default()
                    .src_subresource(subresource)
                    .dst_subresource(subresource)
                    .extent(vk::Extent3D {
                        width: vkctx.swapchain.extent.width,
                        height: vkctx.swapchain.extent.height,
                        depth: 1,
                    });

                vk.cmd_resolve_image(
                    command_buffer,
                    color_image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    present_image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[resolve_region],
                );
                profiler.end(command_buffer, scope);

                // transition to presentable
                vkutils::image_barrier(
                    vk,
                    command_buffer,
                    present_image,
                    (
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::PipelineStageFlags::TRANSFER,
                    ),
                    (
                        vk::ImageLayout::PRESENT_SRC_KHR,
                        vk::AccessFlags::NONE,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    ),
                    color_subresource_range,
                );
            }

            profiler.end(command_buffer, gpu_total_scope);

            vk.end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");

            let acquire_sem = [acq_sem];
            let signal_sem = [fin_sem];
            let command_buffers = [command_buffer];

            let submits = [vk::SubmitInfo::default()
                .wait_semaphores(&acquire_sem)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_sem)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::TRANSFER])];

            vk.reset_fences(&[fence]).expect("Failed fence reset");
            vk.queue_submit(queue, &submits, fence)
                .expect("Failed to submit");

            let present_result =
                vkctx
                    .swapchain
                    .present(image_index.try_into().unwrap(), &[fin_sem], queue);

            match present_result {
                Ok(false) => {
                    if is_swapchain_suboptimal {
                        return FrameOutcome::RebuildSwapchain;
                    } else {
                        return FrameOutcome::Presented;
                    }
                }
                Ok(true) => return FrameOutcome::RebuildSwapchain,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return FrameOutcome::RebuildSwapchain,
                Err(e) => panic!("{e:?}"), // TODO this may return surface lost, which should be easy to handle
            }
        }
    }
}

fn create_draw_mesh_tasks_command_buf(
    ctx: &&mut vkutils::context::VulkanContext,
) -> vkutils::buffer::Buffer {
    let draws: std::vec::Vec<vk::DrawMeshTasksIndirectCommandEXT> =
        vec![vk::DrawMeshTasksIndirectCommandEXT {
            group_count_x: 0,
            group_count_y: 0,
            group_count_z: 0,
        }];

    let buffer = ctx.upload_buffer(
        &draws,
        vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::INDIRECT_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    buffer
}

fn create_render_target_image(ctx: &vkutils::context::VulkanContext) -> vkutils::image::Image {
    let format = ctx.swapchain.surface_format.format;
    let extent = ctx.swapchain.extent;

    ctx.create_image(
        format,
        extent,
        1,
        vk::SampleCountFlags::TYPE_8,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
        vk::ImageAspectFlags::COLOR,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
}

fn create_depth_image(ctx: &vkutils::context::VulkanContext) -> vkutils::image::Image {
    let extent = ctx.swapchain.extent;
    ctx.create_image(
        ctx.depth_format,
        extent,
        1,
        vk::SampleCountFlags::TYPE_8,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::ImageAspectFlags::DEPTH,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
}
