use ash::vk;

use super::vk_destroy::VkDestroy;

pub struct CommandPool {
    pub handle: vk::CommandPool,
    command_buffers: std::vec::Vec<vk::CommandBuffer>,
    device: ash::Device,
}

impl CommandPool {
    pub fn new(
        device: ash::Device,
        flags: vk::CommandPoolCreateFlags,
        queue_family_index: u32,
    ) -> Self {
        let command_pool_create_info = vk::CommandPoolCreateInfo {
            flags,
            queue_family_index,
            ..Default::default()
        };

        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }
            .expect("Failed to create command pool");

        Self {
            handle: command_pool,
            command_buffers: vec![],
            device,
        }
    }

    pub fn allocate_command_buffers(
        &mut self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> std::vec::Vec<vk::CommandBuffer> {
        let command_buffers = self.allocate_unmanaged_command_buffers(level, count);

        self.command_buffers.extend(&command_buffers);

        command_buffers
    }

    fn allocate_unmanaged_command_buffers(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> std::vec::Vec<vk::CommandBuffer> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: self.handle,
            level,
            command_buffer_count: count,
            ..Default::default()
        };

        let command_buffers = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)
        }
        .expect("Failed to allocate command buffer");

        command_buffers
    }

    fn free_unmanaged_command_buffer(&self, command_buffer: vk::CommandBuffer) {
        let cmd_bfrs = [command_buffer];
        unsafe {
            self.device.free_command_buffers(self.handle, &cmd_bfrs);
        }
    }

    pub fn execute_short_lived_command_buffer<F>(&self, queue: vk::Queue, record_cmd_buffer: F)
    where
        F: FnOnce(ash::Device, vk::CommandBuffer),
    {
        let cmd_buffer =
            self.allocate_unmanaged_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)[0];

        let begin_info = vk::CommandBufferBeginInfo::default();

        unsafe {
            self.device
                .begin_command_buffer(cmd_buffer, &begin_info)
                .expect("Failed to begin command buffer")
        };

        record_cmd_buffer(self.device.clone(), cmd_buffer);

        unsafe {
            self.device
                .end_command_buffer(cmd_buffer)
                .expect("Faild to end command buffer")
        };

        let cmd_buffers = [cmd_buffer];
        let submits = [vk::SubmitInfo::default().command_buffers(&cmd_buffers)];
        let fence = super::fence::new(self.device.clone(), false);

        unsafe {
            self.device
                .queue_submit(queue, &submits, fence.handle)
                .expect("Failed to submit queue");

            self.device
                .wait_for_fences(&[fence.handle], true, 10000000000)
                .expect("Error waiting for fences");
        };

        fence.vk_destroy();

        self.free_unmanaged_command_buffer(cmd_buffer);
    }

    pub fn transition_image_layout(
        &self,
        queue: vk::Queue,
        image: vk::Image,
        src: (vk::ImageLayout, vk::AccessFlags, vk::PipelineStageFlags),
        dst: (vk::ImageLayout, vk::AccessFlags, vk::PipelineStageFlags),
        subresource_range: vk::ImageSubresourceRange,
    ) {
        self.execute_short_lived_command_buffer(queue, |device, command_buffer| {
            super::image_barrier(&device, command_buffer, image, src, dst, subresource_range);
        });
    }
}

impl super::vk_destroy::VkDestroy for CommandPool {
    fn vk_destroy(&self) {
        unsafe {
            if !self.command_buffers.is_empty() {
                self.device
                    .free_command_buffers(self.handle, self.command_buffers.as_slice());
            }
            self.device.destroy_command_pool(self.handle, None);
        }
    }
}
