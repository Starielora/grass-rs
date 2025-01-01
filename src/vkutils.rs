use std::mem::size_of;

use ash::{vk, Entry};

use crate::camera::GPUCameraData;
use crate::vkutils_new;
use crate::vkutils_new::vk_destroy::VkDestroy;

pub struct Context {
    #[allow(dead_code)]
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    debug_utils: vkutils_new::debug_utils::DebugUtils,
    pub swapchain: vkutils_new::swapchain::Swapchain,
    pub physical_device: vkutils_new::physical_device::PhysicalDevice,
    pub device: ash::Device,
    pub present_queue: vk::Queue,
    pub color_image: vkutils_new::image::Image,
    pub depth_image: vkutils_new::image::Image,
    pub camera_buffer: vkutils_new::buffer::Buffer,
    pub command_pool: vkutils_new::command_pool::CommandPool,
    pub transient_graphics_command_pool: vkutils_new::command_pool::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub acquire_semaphore: vkutils_new::semaphore::Semaphore,
    pub wait_semaphore: vkutils_new::semaphore::Semaphore,
    pub descriptor_set: vkutils_new::descriptor_set::bindless::DescriptorSet,
    pub render_finished_semaphore: vkutils_new::semaphore::Semaphore,
    pub gui_finished_semaphore: vkutils_new::semaphore::Semaphore,
    pub copy_finished_semaphore: vkutils_new::semaphore::Semaphore,
}

impl Context {
    pub fn new(window: &winit::window::Window) -> Context {
        let entry = unsafe { Entry::load().expect("Could not find Vulkan.") };

        let instance = vkutils_new::instance::create(&entry);
        let debug_utils = vkutils_new::debug_utils::DebugUtils::new(&entry, &instance);

        let physical_device = vkutils_new::physical_device::find_suitable(&instance);
        let queue_indices = vec![
            physical_device.graphics_queue_family_index,
            physical_device.compute_queue_family_index,
            physical_device.transfer_queue_family_index,
        ];

        let device = vkutils_new::device::create(&instance, physical_device.handle, &queue_indices);

        let descriptor_set =
            vkutils_new::descriptor_set::bindless::DescriptorSet::new(device.clone());

        let queues = vkutils_new::device_queue::get_device_queues(&device, &queue_indices);

        let present_queue = queues[0];
        let _transfer_queue = queues[1];
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
            &device,
            vk::ImageCreateFlags::empty(),
            swapchain.surface_format.format,
            swapchain.extent.clone(),
            1,
            vk::SampleCountFlags::TYPE_1,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::ImageLayout::UNDEFINED,
            vk::ImageAspectFlags::COLOR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device.memory_props,
        );

        let depth_image = vkutils_new::image::Image::new(
            &device,
            vk::ImageCreateFlags::empty(),
            vk::Format::D32_SFLOAT, // todo query this from device
            swapchain.extent.clone(),
            1,
            vk::SampleCountFlags::TYPE_1,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageAspectFlags::DEPTH,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device.memory_props,
        );

        let camera_data_buffer = vkutils_new::buffer::Buffer::new(
            &device,
            size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            // BAR buffer
            vk::MemoryPropertyFlags::HOST_VISIBLE
                | vk::MemoryPropertyFlags::HOST_COHERENT
                | vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device.props,
            &physical_device.memory_props,
        );

        let transient_graphics_command_pool = vkutils_new::command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::TRANSIENT,
            physical_device.graphics_queue_family_index,
        );

        let mut command_pool = vkutils_new::command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            physical_device.graphics_queue_family_index,
        );

        let command_buffers =
            command_pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 5); // two for swapchain, one for image copy at the end, one for imgui

        let acquire_semaphore = vkutils_new::semaphore::new(device.clone());
        let wait_semaphore = vkutils_new::semaphore::new(device.clone());
        let render_finished_semaphore = vkutils_new::semaphore::new(device.clone());
        let gui_finished_semaphore = vkutils_new::semaphore::new(device.clone());
        let copy_finished_semaphore = vkutils_new::semaphore::new(device.clone());

        Self {
            entry,
            instance,
            debug_utils,
            physical_device,
            device,
            present_queue,
            swapchain,
            color_image,
            depth_image,
            camera_buffer: camera_data_buffer,
            command_pool,
            transient_graphics_command_pool,
            command_buffers,
            acquire_semaphore,
            wait_semaphore,
            descriptor_set,
            render_finished_semaphore,
            gui_finished_semaphore,
            copy_finished_semaphore,
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
            &self.device,
            size,
            usage,
            memory_property_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
    }

    pub fn create_buffer(
        self: &Self,
        size: usize,
        usage: vk::BufferUsageFlags,
        memory_propery_flags: vk::MemoryPropertyFlags,
    ) -> vkutils_new::buffer::Buffer {
        vkutils_new::buffer::Buffer::new(
            &self.device,
            size,
            usage,
            memory_propery_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
    }

    pub fn image_barrier2(
        self: &Self,
        command_buffer: vk::CommandBuffer,
        image: vk::Image,
        old_image_layout: vk::ImageLayout,
        new_image_layout: vk::ImageLayout,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        subresource_range: vk::ImageSubresourceRange,
    ) {
        let memory_barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .old_layout(old_image_layout)
            .new_layout(new_image_layout)
            .image(image)
            .subresource_range(subresource_range);

        let mem_barriers = [];
        let buffer_barriers = [];
        let image_barriers = [memory_barrier];

        unsafe {
            self.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &mem_barriers,
                &buffer_barriers,
                &image_barriers,
            );
        }
    }
}

pub fn execute_short_lived_command_buffer<F>(
    device: ash::Device,
    command_pool: &mut vkutils_new::command_pool::CommandPool,
    queue: vk::Queue,
    record_cmd_buffer: F,
) where
    F: FnOnce(ash::Device, vk::CommandBuffer),
{
    let cmd_buffer = command_pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)[0];

    let begin_info = vk::CommandBufferBeginInfo::default();

    unsafe {
        device
            .begin_command_buffer(cmd_buffer, &begin_info)
            .expect("Failed to begin command buffer")
    };

    record_cmd_buffer(device.clone(), cmd_buffer);

    unsafe {
        device
            .end_command_buffer(cmd_buffer)
            .expect("Faild to end command buffer")
    };

    let cmd_buffers = [cmd_buffer];
    let submits = [vk::SubmitInfo::default().command_buffers(&cmd_buffers)];
    let fence = vkutils_new::fence::new(device.clone(), false);

    unsafe {
        device
            .queue_submit(queue, &submits, fence.handle)
            .expect("Failed to submit queue");

        device
            .wait_for_fences(&[fence.handle], true, 10000000000)
            .expect("Error waiting for fences");
    };

    fence.vk_destroy();

    command_pool.free_command_buffer(cmd_buffer);
}

pub fn image_barrier(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    old_image_layout: vk::ImageLayout,
    new_image_layout: vk::ImageLayout,
    src_stage_mask: vk::PipelineStageFlags,
    dst_stage_mask: vk::PipelineStageFlags,
    subresource_range: vk::ImageSubresourceRange,
) {
    let mut memory_barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_image_layout)
        .new_layout(new_image_layout)
        .image(image)
        .subresource_range(subresource_range);

    match old_image_layout {
        vk::ImageLayout::UNDEFINED => memory_barrier.src_access_mask = vk::AccessFlags::NONE,
        vk::ImageLayout::PREINITIALIZED => {
            memory_barrier.src_access_mask = vk::AccessFlags::HOST_WRITE
        }
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
            memory_barrier.src_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE
        }
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
            memory_barrier.src_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
        }
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => {
            memory_barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ
        }
        vk::ImageLayout::TRANSFER_DST_OPTIMAL => {
            memory_barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE
        }
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
            memory_barrier.src_access_mask = vk::AccessFlags::SHADER_READ
        }
        vk::ImageLayout::PRESENT_SRC_KHR => {
            memory_barrier.src_access_mask = vk::AccessFlags::empty();
        }
        _ => todo!("TBD"),
    }

    match new_image_layout {
        vk::ImageLayout::TRANSFER_DST_OPTIMAL => {
            memory_barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE
        }
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => {
            memory_barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ
        }
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
            memory_barrier.dst_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE
        }
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
            memory_barrier.dst_access_mask =
                memory_barrier.dst_access_mask | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
        }
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
            if memory_barrier.src_access_mask == vk::AccessFlags::NONE {
                memory_barrier.src_access_mask =
                    vk::AccessFlags::HOST_WRITE | vk::AccessFlags::TRANSFER_WRITE;
            }
            memory_barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
        }
        vk::ImageLayout::PRESENT_SRC_KHR => {
            memory_barrier.dst_access_mask = vk::AccessFlags::empty();
        }
        _ => todo!("TBD"),
    }

    let mem_barriers = [];
    let buffer_barriers = [];
    let image_barriers = [memory_barrier];

    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage_mask,
            dst_stage_mask,
            vk::DependencyFlags::empty(),
            &mem_barriers,
            &buffer_barriers,
            &image_barriers,
        );
    }
}

impl std::ops::Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.gui_finished_semaphore.vk_destroy();
            self.copy_finished_semaphore.vk_destroy();
            self.render_finished_semaphore.vk_destroy();
            self.wait_semaphore.vk_destroy();
            self.acquire_semaphore.vk_destroy();
            self.command_pool.vk_destroy();
            self.transient_graphics_command_pool.vk_destroy();
            self.descriptor_set.vk_destroy();
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
