use crate::assets::gltf_asset;
use crate::camera::GPUCameraData;
use crate::frustum2::Frustum2;
use crate::grid2::Grid2;
use crate::meshlet2::{self};
use crate::skybox2::Skybox2;
use crate::vkutils::{self, vk_destroy::VkDestroy};
use ash::vk;
use glm;
use rand::RngExt;

pub struct Renderer2 {
    vk: ash::Device,
    command_buffers: std::vec::Vec<vk::CommandBuffer>,

    render_target: vkutils::image::Image,
    depth_image: vkutils::image::Image,

    // TODO maybe don't spit such small buffers into multiple - use one backing memory
    view_camera_data_buffer: vkutils::buffer::Buffer,
    cull_camera_data_buffer: vkutils::buffer::Buffer,
    grid: Grid2,
    skybox: Skybox2,
    frustum: Frustum2,
    frustum_enabled: bool,
    bounding_sphere: meshlet2::bounding_sphere::BoundingSphere,

    meshlet_pipeline: meshlet2::GraphicsPipeline,
    geometry_data: meshlet2::GeometryBuffers,
    draw_params_buf: vkutils::buffer::Buffer,

    render_finished_semaphore: vk::Semaphore,
}

impl std::ops::Drop for Renderer2 {
    fn drop(&mut self) {
        let vk = &self.vk;
        unsafe {
            self.draw_params_buf.vk_destroy();
            self.render_target.vk_destroy();
            self.depth_image.vk_destroy();
            self.view_camera_data_buffer.vk_destroy();
            self.cull_camera_data_buffer.vk_destroy();
            self.geometry_data.vk_destroy();
            vk.destroy_semaphore(self.render_finished_semaphore, None);
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
            ctx.swapchain.images.len().try_into().unwrap(),
        );

        let render_target = create_render_target_image(&ctx);
        let depth_image = create_depth_image(&ctx);

        let render_finished_semaphore = ctx.create_semaphore_vk();

        let view_camera_data_buffer = ctx.create_bar_buffer(
            size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );
        let cull_camera_data_buffer = ctx.create_bar_buffer(
            size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let view_camera_bda = view_camera_data_buffer.device_address.unwrap();
        let cull_camera_bda = cull_camera_data_buffer.device_address.unwrap();

        let grid = Grid2::new(
            &ctx.device,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            ctx.bindless_descriptor_set.layout,
            view_camera_bda,
        )
        .expect("Failed to instantiate Grid object");

        let skybox = Skybox2::new(&ctx, view_camera_data_buffer.device_address.unwrap())
            .expect("Failed to instantiate skybox");

        let frustum = Frustum2::new(
            &ctx.device,
            ctx.bindless_descriptor_set.layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            view_camera_bda,
            cull_camera_bda,
        );

        let bounding_sphere = meshlet2::bounding_sphere::BoundingSphere::new(
            &ctx.device,
            &ctx.mesh_shader_device,
            ctx.bindless_descriptor_set.layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            view_camera_data_buffer.device_address.unwrap(),
            ctx.physical_device.subgroup_size,
            ctx.physical_device.max_task_workgroup_count,
        );

        let brabon_data = gltf_asset::GltfAssetData::new(
            "/home/starielora/dev/repos/Vulkan-Assets/models/chinesedragon.gltf",
        );

        let mut geometry_builder = meshlet2::gltf::GeometryBuilder::new();
        geometry_builder.add_instance(&brabon_data, glm::Mat4::identity(), Option::None);

        {
            let mut rng = rand::rng();

            for _i in 0..10 {
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
        let max_task_workgroup_count = ctx.physical_device.max_task_workgroup_count;
        let (draw_params_buf, draws_count) = create_draw_params_buf(
            &ctx,
            &geometry_data,
            subgroup_size,
            max_task_workgroup_count[0],
        );
        let meshlet_pipeline = meshlet2::GraphicsPipeline::new(
            &ctx.device,
            &ctx.mesh_shader_device,
            ctx.bindless_descriptor_set.layout,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            view_camera_data_buffer.device_address.unwrap(),
            draw_params_buf.handle,
            draw_params_buf.device_address.unwrap(),
            draws_count,
            subgroup_size,
        );

        Self {
            vk: ctx.device.clone(),
            command_buffers,
            render_target,
            depth_image,
            view_camera_data_buffer,
            cull_camera_data_buffer,
            grid,
            skybox,
            frustum,
            frustum_enabled: true,
            bounding_sphere,
            meshlet_pipeline,
            geometry_data,
            draw_params_buf,
            render_finished_semaphore,
        }
    }

    pub fn toggle_frustum(&mut self) {
        self.frustum_enabled = !self.frustum_enabled;
    }

    pub fn toggle_bounding_sphere_mode(&mut self) {
        self.bounding_sphere.toggle_mode();
    }

    pub fn incr_lod(&mut self) {
        self.meshlet_pipeline.lod += 1;
        println!("Current lod: {}", self.meshlet_pipeline.lod);
    }

    pub fn decr_lod(&mut self) {
        self.meshlet_pipeline.lod -= 1;
        println!("Current lod: {}", self.meshlet_pipeline.lod);
    }

    // TODO make type safe - don't rely on tuple indices - easy to mix
    pub fn update_gpu_camera_data(
        &self,
        view_camera_data: (glm::Vec4, glm::Mat4, glm::Mat4),
        cull_camera_data: (glm::Vec4, glm::Mat4, glm::Mat4),
    ) {
        self.view_camera_data_buffer
            .update_contents(&[GPUCameraData {
                pos: view_camera_data.0,
                projview: view_camera_data.1,
                view: view_camera_data.2,
            }]);
        self.cull_camera_data_buffer
            .update_contents(&[GPUCameraData {
                pos: cull_camera_data.0,
                projview: cull_camera_data.1,
                view: cull_camera_data.2,
            }]);
    }

    pub fn resize(&mut self, ctx: &vkutils::context::VulkanContext) {
        let render_target = create_render_target_image(&ctx);
        let depth_image = create_depth_image(&ctx);

        self.render_target.vk_destroy();
        self.depth_image.vk_destroy();

        self.render_target = render_target;
        self.depth_image = depth_image;
    }

    pub fn draw(&self, vkctx: &mut vkutils::context::VulkanContext) -> FrameOutcome {
        let (acquire_result, acquire_semaphore) =
            vkctx.swapchain.acquire_next_image(!0, vk::Fence::null());

        let (image_index, is_swapchain_suboptimal) = match acquire_result {
            Ok(v) => v,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return FrameOutcome::RebuildSwapchain,
            Err(e) => panic!("{e:?}"),
        };

        let image_index: usize = image_index.try_into().unwrap();

        let queue = vkctx.graphics_present_queue;
        let command_buffer = self.command_buffers[image_index];
        let vk = &self.vk;

        unsafe {
            vk.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("Failed to reset command buffer");

            let begin_info = vk::CommandBufferBeginInfo {
                ..Default::default()
            };
            vk.begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer");

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

            let (color_image, color_image_view) =
                (self.render_target.handle, self.render_target.view);
            let (depth_image, depth_image_view) = (self.depth_image.handle, self.depth_image.view);

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

            self.meshlet_pipeline.record(
                command_buffer,
                vkctx.swapchain.extent,
                &self.geometry_data,
            );

            self.skybox.record(command_buffer, vkctx.swapchain.extent);

            self.bounding_sphere.record(
                command_buffer,
                vkctx.swapchain.extent,
                &self.geometry_data,
                self.meshlet_pipeline.lod,
            );

            if self.frustum_enabled {
                self.frustum.record(command_buffer, vkctx.swapchain.extent);
            }
            self.grid.record(command_buffer, vkctx.swapchain.extent);

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
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                    ),
                    (
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::PipelineStageFlags::TRANSFER,
                    ),
                    color_subresource_range,
                );

                // MSAA resolve
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

            vk.end_command_buffer(command_buffer)
                .expect("Failed to end command buffer");

            let render_finished_semaphore = self.render_finished_semaphore;
            let acquire_sem = [acquire_semaphore];
            let signal_sem = [render_finished_semaphore];
            let command_buffers = [command_buffer];

            let submits = [vk::SubmitInfo::default()
                .wait_semaphores(&acquire_sem)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_sem)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])];

            vk.queue_submit(queue, &submits, vk::Fence::null())
                .expect("Failed to submit");

            let present_result = vkctx.swapchain.present(
                image_index.try_into().unwrap(),
                &[render_finished_semaphore],
                queue,
            );

            vkctx.device.device_wait_idle().expect("Failed to wait");

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

fn create_draw_params_buf(
    ctx: &&mut vkutils::context::VulkanContext,
    geometry_data: &meshlet2::GeometryBuffers,
    subgroup_size: u32,
    max_dim: u32,
) -> (vkutils::buffer::Buffer, u32) {
    let (group_count_x, group_count_y) = meshlet2::gpu::task_dispatch_2d(
        geometry_data.meshlet_instances_count,
        subgroup_size,
        max_dim,
    );
    let draws: std::vec::Vec<vk::DrawMeshTasksIndirectCommandEXT> =
        vec![vk::DrawMeshTasksIndirectCommandEXT {
            group_count_x,
            group_count_y,
            group_count_z: 1,
        }];

    let buffer = ctx.upload_buffer(
        &draws,
        vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::INDIRECT_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    (buffer, draws.len() as u32)
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
