use ash::vk;

use super::vk_destroy::VkDestroy;

pub struct Pipeline {
    device: ash::Device,
    pub handle: vk::Pipeline,
}

impl Pipeline {
    pub fn new(device: ash::Device, pipeline: vk::Pipeline) -> Self {
        Self {
            device,
            handle: pipeline,
        }
    }
}

impl super::vk_destroy::VkDestroy for Pipeline {
    fn vk_destroy(&self) {
        unsafe {
            self.device.destroy_pipeline(self.handle, None);
        }
    }
}

impl std::ops::Drop for Pipeline {
    fn drop(&mut self) {
        self.vk_destroy();
    }
}
