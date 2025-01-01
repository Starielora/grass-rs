use ash::vk;

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
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: self.handle,
            level,
            command_buffer_count: count, // two for swapchain, one for image copy at the end, one for imgui
            ..Default::default()
        };

        let command_buffers = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)
        }
        .expect("Failed to allocate command buffer");

        self.command_buffers.extend(&command_buffers);

        command_buffers
    }

    pub fn free_command_buffer(&mut self, command_buffer: vk::CommandBuffer) {
        let pos_to_remove = self
            .command_buffers
            .iter()
            .position(|&cmdbuf| cmdbuf == command_buffer)
            .unwrap_or_else(|| panic!("Failed to find command buffer in this pool."));

        self.command_buffers.remove(pos_to_remove);

        let cmd_bfrs = [command_buffer];
        unsafe {
            self.device.free_command_buffers(self.handle, &cmd_bfrs);
        }
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
