use std::mem::size_of;

use ash::{vk, Entry};

use crate::camera::GPUCameraData;
use crate::vkutils_new::push_constants::GPUPushConstants;
use crate::vkutils_new::vk_destroy::VkDestroy;
use crate::{depth_map_display_pipeline, vkutils_new};

pub struct Context {
    #[allow(dead_code)]
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    debug_utils: vkutils_new::debug_utils::DebugUtils,
    pub swapchain: vkutils_new::swapchain::Swapchain,
    pub physical_device: vkutils_new::physical_device::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_present_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
    pub color_image: vkutils_new::image::Image,
    pub depth_image: vkutils_new::image::Image,
    pub camera_buffer: vkutils_new::buffer::Buffer,
    pub graphics_command_pool: vkutils_new::command_pool::CommandPool,
    pub transient_transfer_command_pool: vkutils_new::command_pool::CommandPool,
    pub transient_graphics_command_pool: vkutils_new::command_pool::CommandPool,
    pub light_pov_command_buffer: [vk::CommandBuffer; 2],
    pub scene_command_buffer: [vk::CommandBuffer; 2],
    pub depth_display_command_buffer: [vk::CommandBuffer; 2],
    pub imgui_command_buffer: vk::CommandBuffer,
    pub acquire_semaphore: vkutils_new::semaphore::Semaphore,
    pub wait_semaphore: vkutils_new::semaphore::Semaphore,
    pub bindless_descriptor_set: vkutils_new::descriptor_set::bindless::DescriptorSet,
    pub draw_depth_finished_semaphore: vkutils_new::semaphore::Semaphore,
    pub render_finished_semaphore: vkutils_new::semaphore::Semaphore,
    pub gui_finished_semaphore: vkutils_new::semaphore::Semaphore,
}

impl Context {
    pub fn new(window: &winit::window::Window) -> Context {
        let entry = unsafe { Entry::load().expect("Could not find Vulkan.") };

        let instance = vkutils_new::instance::create(&entry);
        let debug_utils = vkutils_new::debug_utils::DebugUtils::new(&entry, &instance);

        let physical_device = vkutils_new::physical_device::find_suitable(&instance);
        let queue_indices = vec![
            physical_device.graphics_queue_family_index,
            physical_device.transfer_queue_family_index,
            physical_device.compute_queue_family_index,
        ];

        let device = vkutils_new::device::create(&instance, physical_device.handle, &queue_indices);

        let descriptor_set =
            vkutils_new::descriptor_set::bindless::DescriptorSet::new(device.clone());

        let queues = vkutils_new::device_queue::get_device_queues(&device, &queue_indices);

        let graphics_present_queue = queues[0];
        let transfer_queue = queues[1];
        let _compute_queue = queues[2];

        let swapchain = vkutils_new::swapchain::Swapchain::new(
            &window,
            &entry,
            &device,
            physical_device.handle,
            &instance,
            physical_device.graphics_queue_family_index,
        );

        let color_image = vkutils_new::image::Image::new(
            device.clone(),
            vk::ImageCreateFlags::empty(),
            swapchain.surface_format.format,
            swapchain.extent.clone(),
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::ImageAspectFlags::COLOR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device.memory_props,
        );

        let depth_image = vkutils_new::image::Image::new(
            device.clone(),
            vk::ImageCreateFlags::empty(),
            vk::Format::D32_SFLOAT, // todo query this from device
            swapchain.extent.clone(),
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::ImageAspectFlags::DEPTH,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device.memory_props,
        );

        let camera_data_buffer = vkutils_new::buffer::Buffer::new(
            device.clone(),
            size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            // BAR buffer
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT
                | vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device.props,
            &physical_device.memory_props,
        );

        let transient_transfer_command_pool = vkutils_new::command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::TRANSIENT,
            physical_device.transfer_queue_family_index,
        );
        let mut transient_graphics_command_pool = vkutils_new::command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::TRANSIENT
                | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            physical_device.graphics_queue_family_index,
        );

        let mut command_pool = vkutils_new::command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            physical_device.graphics_queue_family_index,
        );

        let graphics_command_buffers =
            command_pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 6);
        let imgui_command_buffer = transient_graphics_command_pool
            .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)[0]; // This one is reset and recorded each frame

        let acquire_semaphore = vkutils_new::semaphore::new(device.clone());
        let wait_semaphore = vkutils_new::semaphore::new(device.clone());
        let render_finished_semaphore = vkutils_new::semaphore::new(device.clone());
        let gui_finished_semaphore = vkutils_new::semaphore::new(device.clone());
        let draw_depth_finished_semaphore = vkutils_new::semaphore::new(device.clone());

        Self {
            entry,
            instance,
            debug_utils,
            physical_device,
            device,
            graphics_present_queue,
            transfer_queue,
            swapchain,
            color_image,
            depth_image,
            camera_buffer: camera_data_buffer,
            graphics_command_pool: command_pool,
            transient_transfer_command_pool,
            transient_graphics_command_pool,
            light_pov_command_buffer: [graphics_command_buffers[2], graphics_command_buffers[3]],
            scene_command_buffer: [graphics_command_buffers[0], graphics_command_buffers[1]],
            depth_display_command_buffer: [
                graphics_command_buffers[4],
                graphics_command_buffers[5],
            ],
            imgui_command_buffer,
            acquire_semaphore,
            wait_semaphore,
            bindless_descriptor_set: descriptor_set,
            render_finished_semaphore,
            gui_finished_semaphore,
            draw_depth_finished_semaphore,
        }
    }

    // create Base Address Register (BAR) buffer.
    // DEVICE_LOCAL, HOST_VISIBLE (mappable), HOST_COHERENT
    pub fn create_bar_buffer(
        self: &Self,
        size: usize,
        usage: vk::BufferUsageFlags,
    ) -> vkutils_new::buffer::Buffer {
        let memory_property_flags = vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT
            | vk::MemoryPropertyFlags::DEVICE_LOCAL;

        vkutils_new::buffer::Buffer::new(
            self.device.clone(),
            size,
            usage,
            memory_property_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
    }

    pub fn create_bar_storage_buffer(self: &Self, size: usize) -> vkutils_new::buffer::Buffer {
        let usage =
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;

        self.create_bar_buffer(size, usage)
    }

    pub fn create_buffer(
        self: &Self,
        size: usize,
        usage: vk::BufferUsageFlags,
        memory_propery_flags: vk::MemoryPropertyFlags,
    ) -> vkutils_new::buffer::Buffer {
        vkutils_new::buffer::Buffer::new(
            self.device.clone(),
            size,
            usage,
            memory_propery_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
    }

    fn begin_scene_rendering(&self, command_buffer: vk::CommandBuffer) {
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
            .image_view(self.color_image.view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(color_clear_value)];

        let depth_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(self.depth_image.view)
            .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(depth_clear_value);

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                extent: self.swapchain.extent,
                offset: vk::Offset2D { x: 0, y: 0 },
            })
            .layer_count(1)
            .color_attachments(&color_attachments)
            .depth_attachment(&depth_attachment);

        unsafe {
            self.device
                .cmd_begin_rendering(command_buffer, &rendering_info);
        }
    }

    fn record_image_barriers_for_scene_rendering(&self, command_buffer: vk::CommandBuffer) {
        let color_subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(vk::REMAINING_ARRAY_LAYERS);

        vkutils_new::image_barrier(
            &self.device,
            command_buffer,
            self.color_image.handle,
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

        vkutils_new::image_barrier(
            &self.device,
            command_buffer,
            self.depth_image.handle,
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
            vkutils_new::depth_subresource_range(),
        );
    }

    pub fn render_scene(
        &self,
        skybox: &crate::skybox::Skybox,
        grid: &crate::grid::Grid,
        push_constants: &mut GPUPushConstants,
        pipeline: vk::Pipeline,
        meshes: &[std::rc::Rc<std::cell::RefCell<dyn crate::drawable::Drawable>>],
    ) {
        for command_buffer in self.scene_command_buffer {
            let begin_info = vk::CommandBufferBeginInfo {
                ..Default::default()
            };
            unsafe {
                self.device
                    .begin_command_buffer(command_buffer, &begin_info)
                    .expect("Failed to begin command buffer");
            }

            self.record_image_barriers_for_scene_rendering(command_buffer);
            self.begin_scene_rendering(command_buffer);

            skybox.record(command_buffer, push_constants);

            crate::mesh::pipeline::color::record(
                &self.device,
                command_buffer,
                pipeline,
                push_constants,
                &meshes,
            );

            grid.record(command_buffer, push_constants);

            unsafe {
                self.device.cmd_end_rendering(command_buffer);
                self.device
                    .end_command_buffer(command_buffer)
                    .expect("Failed to end command buffer???");
            }
        }
    }

    pub fn render_shadow_map(
        &self,
        push_constants: &mut GPUPushConstants,
        pipeline: vk::Pipeline,
        dir_light_depth_image: (vk::Image, vk::ImageView),
        dir_light_camera_buffer_device_address: vk::DeviceAddress,
        meshes: &[std::rc::Rc<std::cell::RefCell<dyn crate::drawable::Drawable>>],
    ) {
        for command_buffer in self.light_pov_command_buffer {
            // TODO this is a bit too self-contained probably
            crate::mesh::pipeline::shadow_map::record(
                &self.device,
                command_buffer,
                pipeline,
                self.swapchain.extent,
                dir_light_depth_image,
                dir_light_camera_buffer_device_address,
                push_constants,
                &meshes,
            );
        }
    }

    pub fn render_shadow_map_to_swapchain_image(
        &self,
        depth_map_display_pipeline: &depth_map_display_pipeline::DepthMapDisplayPipeline,
        dir_light_depth_image: (vk::Image, vk::ImageView),
    ) {
        // TODO ugh
        let swapchain_images = [
            (self.swapchain.images[0], self.swapchain.views[0]),
            (self.swapchain.images[1], self.swapchain.views[1]),
        ];

        for (&command_buffer, swapchain_image) in self
            .depth_display_command_buffer
            .iter()
            .zip(swapchain_images)
        {
            depth_map_display_pipeline.record(
                command_buffer,
                self.bindless_descriptor_set.handle,
                dir_light_depth_image,
                (self.color_image.handle, self.color_image.view),
                swapchain_image,
                self.swapchain.extent,
            );
        }
    }

    pub fn submit_and_present_scene(&self, image_index: u32) {
        // TODO these can be prebuilt per (double buffering) image as well
        let acquire_semaphores = [self.acquire_semaphore.handle];
        let shadow_map_command_buffers = [self.light_pov_command_buffer[image_index as usize]];
        let shadow_map_finished_semaphores = [self.draw_depth_finished_semaphore.handle];
        let render_semaphores = [self.render_finished_semaphore.handle];
        let gui_command_buffers = [self.imgui_command_buffer];
        let wait_semaphores = [self.wait_semaphore.handle];
        [self.depth_display_command_buffer[image_index as usize]];
        let draw_command_buffers = [self.scene_command_buffer[image_index as usize]];

        let submits = [
            vk::SubmitInfo::default()
                .wait_semaphores(&acquire_semaphores)
                .command_buffers(&shadow_map_command_buffers)
                .signal_semaphores(&shadow_map_finished_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::VERTEX_SHADER]),
            vk::SubmitInfo::default()
                .wait_semaphores(&shadow_map_finished_semaphores)
                .command_buffers(&draw_command_buffers)
                .signal_semaphores(&render_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
            vk::SubmitInfo::default()
                .wait_semaphores(&render_semaphores)
                .command_buffers(&gui_command_buffers)
                .signal_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
        ];
        unsafe {
            self.device
                .queue_submit(self.graphics_present_queue, &submits, vk::Fence::null())
        }
        .expect("Failed to submit");

        self.swapchain
            .present(image_index, &wait_semaphores, self.graphics_present_queue);
    }

    pub fn submit_and_present_shadow_map(&self, image_index: u32) {
        // TODO these can be prebuilt per (double buffering) image as well
        let acquire_semaphores = [self.acquire_semaphore.handle];
        let shadow_map_command_buffers = [self.light_pov_command_buffer[image_index as usize]];
        let shadow_map_finished_semaphores = [self.draw_depth_finished_semaphore.handle];
        let render_semaphores = [self.render_finished_semaphore.handle];
        let gui_command_buffers = [self.imgui_command_buffer];
        let wait_semaphores = [self.wait_semaphore.handle];
        let depth_display_command_buffers =
            [self.depth_display_command_buffer[image_index as usize]];

        let submits = [
            vk::SubmitInfo::default()
                .wait_semaphores(&acquire_semaphores)
                .command_buffers(&shadow_map_command_buffers)
                .signal_semaphores(&shadow_map_finished_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::VERTEX_SHADER]),
            vk::SubmitInfo::default()
                .wait_semaphores(&shadow_map_finished_semaphores)
                .command_buffers(&depth_display_command_buffers)
                .signal_semaphores(&render_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
            vk::SubmitInfo::default()
                .wait_semaphores(&render_semaphores)
                .command_buffers(&gui_command_buffers)
                .signal_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
        ];
        unsafe {
            self.device
                .queue_submit(self.graphics_present_queue, &submits, vk::Fence::null())
        }
        .expect("Failed to submit");

        self.swapchain.present(
            image_index as u32,
            &wait_semaphores,
            self.graphics_present_queue,
        );
    }
}

impl std::ops::Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.draw_depth_finished_semaphore.vk_destroy();
            self.gui_finished_semaphore.vk_destroy();
            self.render_finished_semaphore.vk_destroy();
            self.wait_semaphore.vk_destroy();
            self.acquire_semaphore.vk_destroy();
            self.graphics_command_pool.vk_destroy();
            self.transient_transfer_command_pool.vk_destroy();
            self.transient_graphics_command_pool.vk_destroy();
            self.bindless_descriptor_set.vk_destroy();
            self.camera_buffer.vk_destroy();
            self.depth_image.vk_destroy();
            self.color_image.vk_destroy();
            self.swapchain.vk_destroy();
            self.device.destroy_device(None);
            self.debug_utils.vk_destroy();
            self.instance.destroy_instance(None);
        }
    }
}
