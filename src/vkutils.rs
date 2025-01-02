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
    pub graphics_present_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
    pub color_image: vkutils_new::image::Image,
    pub depth_image: vkutils_new::image::Image,
    pub camera_buffer: vkutils_new::buffer::Buffer,
    pub graphics_command_pool: vkutils_new::command_pool::CommandPool,
    pub transient_transfer_command_pool: vkutils_new::command_pool::CommandPool,
    pub transient_graphics_command_pool: vkutils_new::command_pool::CommandPool,
    pub scene_command_buffer: [vk::CommandBuffer; 2],
    pub imgui_command_buffer: vk::CommandBuffer,
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
            command_pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 2);
        let imgui_command_buffer = transient_graphics_command_pool
            .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)[0]; // This one is reset and recorded each frame

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
            graphics_present_queue,
            transfer_queue,
            swapchain,
            color_image,
            depth_image,
            camera_buffer: camera_data_buffer,
            graphics_command_pool: command_pool,
            transient_transfer_command_pool,
            transient_graphics_command_pool,
            scene_command_buffer: [graphics_command_buffers[0], graphics_command_buffers[1]],
            imgui_command_buffer,
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
            self.device.clone(),
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
            self.device.clone(),
            size,
            usage,
            memory_propery_flags,
            &self.physical_device.props,
            &self.physical_device.memory_props,
        )
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
            self.graphics_command_pool.vk_destroy();
            self.transient_transfer_command_pool.vk_destroy();
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
