use ash::vk;

use super::{
    buffer, command_pool, debug_utils, descriptor_set, device, device_queue, image, instance,
    physical_device, semaphore, swapchain, vk_destroy::VkDestroy,
};

pub struct VulkanContext {
    #[allow(dead_code)]
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    debug_utils: debug_utils::DebugUtils,
    pub swapchain: swapchain::Swapchain,
    pub physical_device: physical_device::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_present_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
    pub bindless_descriptor_set: descriptor_set::bindless::DescriptorSet,
    pub graphics_command_pool: command_pool::CommandPool,
    pub transient_transfer_command_pool: command_pool::CommandPool,
    pub transient_graphics_command_pool: command_pool::CommandPool,
    pub depth_format: vk::Format,
}

impl VulkanContext {
    pub fn new(window: &winit::window::Window) -> VulkanContext {
        let entry = unsafe { ash::Entry::load().expect("Could not find Vulkan.") };

        let instance = instance::create(&entry);
        let debug_utils = debug_utils::DebugUtils::new(&entry, &instance);

        let physical_device = physical_device::find_suitable(&instance);
        let queue_indices = vec![
            physical_device.graphics_queue_family_index,
            physical_device.transfer_queue_family_index,
            physical_device.compute_queue_family_index,
        ];

        let device = device::create(&instance, physical_device.handle, &queue_indices);

        let bindless_descriptor_set = descriptor_set::bindless::DescriptorSet::new(device.clone());

        let queues = device_queue::get_device_queues(&device, &queue_indices);

        let graphics_present_queue = queues[0];
        let transfer_queue = queues[1];
        let _compute_queue = queues[2];

        let swapchain = swapchain::Swapchain::new(
            &window,
            &entry,
            &device,
            physical_device.handle,
            &instance,
            physical_device.graphics_queue_family_index,
        );

        let transient_transfer_command_pool = command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::TRANSIENT,
            physical_device.transfer_queue_family_index,
        );
        let transient_graphics_command_pool = command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::TRANSIENT
                | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            physical_device.graphics_queue_family_index,
        );

        let graphics_command_pool = command_pool::CommandPool::new(
            device.clone(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            physical_device.graphics_queue_family_index,
        );

        Self {
            entry,
            instance,
            debug_utils,
            swapchain,
            physical_device,
            device,
            graphics_present_queue,
            transfer_queue,
            bindless_descriptor_set,
            graphics_command_pool,
            transient_transfer_command_pool,
            transient_graphics_command_pool,
            depth_format: vk::Format::D32_SFLOAT, // TODO query from device,
        }
    }

    // swapchain extent
    pub fn create_image(
        &self,
        format: vk::Format,
        extent: vk::Extent2D,
        array_layers: u32,
        samples: vk::SampleCountFlags,
        usage: vk::ImageUsageFlags,
        aspect_flags: vk::ImageAspectFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> image::Image {
        image::Image::new(
            self.device.clone(),
            vk::ImageCreateFlags::empty(),
            format,
            extent,
            array_layers,
            samples,
            usage,
            aspect_flags,
            memory_property_flags,
            &self.physical_device.memory_props,
        )
    }

    pub fn create_buffer(
        self: &Self,
        size: usize,
        usage: vk::BufferUsageFlags,
        memory_propery_flags: vk::MemoryPropertyFlags,
    ) -> buffer::Buffer {
        buffer::Buffer::new(
            self.device.clone(),
            size,
            usage,
            memory_propery_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
    }

    // create Base Address Register (BAR) buffer.
    // DEVICE_LOCAL, HOST_VISIBLE (mappable), HOST_COHERENT
    pub fn create_bar_buffer(
        self: &Self,
        size: usize,
        usage: vk::BufferUsageFlags,
    ) -> buffer::Buffer {
        let memory_property_flags = vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT
            | vk::MemoryPropertyFlags::DEVICE_LOCAL;

        buffer::Buffer::new(
            self.device.clone(),
            size,
            usage,
            memory_property_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
    }

    pub fn create_semaphore_vk(&self) -> vk::Semaphore {
        semaphore::new_vk(self.device.clone())
    }

    pub fn upload_buffer<T: std::marker::Copy>(
        &self,
        data: &Vec<T>,
        buffer_usage: vk::BufferUsageFlags,
    ) -> buffer::Buffer {
        let mut staging_buffer = self.create_buffer(
            data.len() * std::mem::size_of::<T>(),
            buffer_usage | vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let device_buffer = self.create_buffer(
            data.len() * std::mem::size_of::<T>(),
            buffer_usage | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        staging_buffer.update_contents(&data.as_slice());
        staging_buffer.unmap_memory();

        self.transient_transfer_command_pool
            .execute_short_lived_command_buffer(self.transfer_queue, |device, command_buffer| {
                let region = [vk::BufferCopy::default().size(staging_buffer.allocation_size)];
                unsafe {
                    device.cmd_copy_buffer(
                        command_buffer,
                        staging_buffer.handle,
                        device_buffer.handle,
                        &region,
                    );
                }
            });

        staging_buffer.vk_destroy();

        device_buffer
    }
}

impl std::ops::Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.graphics_command_pool.vk_destroy();
            self.transient_transfer_command_pool.vk_destroy();
            self.transient_graphics_command_pool.vk_destroy();
            self.bindless_descriptor_set.vk_destroy();
            self.swapchain.vk_destroy();
            self.device.destroy_device(None);
            self.debug_utils.vk_destroy();
            self.instance.destroy_instance(None);
        }
    }
}
