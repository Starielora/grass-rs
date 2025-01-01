use ash::vk;

use super::vk_destroy;

pub struct Fence {
    pub handle: vk::Fence,
    pub device: ash::Device,
}

pub fn new(device: ash::Device, signaled: bool) -> Fence {
    Fence::new(device, signaled)
}

impl Fence {
    pub fn new(device: ash::Device, signaled: bool) -> Self {
        let handle = {
            let create_info = vk::FenceCreateInfo {
                flags: match signaled {
                    true => vk::FenceCreateFlags::SIGNALED,
                    false => vk::FenceCreateFlags::empty(),
                },
                ..Default::default()
            };

            unsafe { device.create_fence(&create_info, None) }.expect("Failed to create fence")
        };

        Self {
            handle,
            device: device.clone(),
        }
    }
}

impl vk_destroy::VkDestroy for Fence {
    fn vk_destroy(&self) {
        unsafe {
            self.device.destroy_fence(self.handle, None);
        }
    }
}
