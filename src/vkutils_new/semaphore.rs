use ash::vk;

use super::vk_destroy;

pub struct Semaphore {
    pub handle: vk::Semaphore,
    pub device: ash::Device,
}

pub fn new(device: ash::Device) -> Semaphore {
    Semaphore::new(device)
}

pub fn new_vk(device: ash::Device) -> vk::Semaphore {
    let create_info = vk::SemaphoreCreateInfo {
        ..Default::default()
    };

    unsafe { device.create_semaphore(&create_info, None) }.expect("Failed to create semaphore")
}

impl Semaphore {
    pub fn new(device: ash::Device) -> Self {
        let handle = {
            let create_info = vk::SemaphoreCreateInfo {
                ..Default::default()
            };

            unsafe { device.create_semaphore(&create_info, None) }
                .expect("Failed to create semaphore")
        };

        Self {
            handle,
            device: device.clone(),
        }
    }
}

impl vk_destroy::VkDestroy for Semaphore {
    fn vk_destroy(&self) {
        unsafe {
            self.device.destroy_semaphore(self.handle, None);
        }
    }
}
