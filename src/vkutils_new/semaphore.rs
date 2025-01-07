use ash::vk;

pub fn new_vk(device: ash::Device) -> vk::Semaphore {
    let create_info = vk::SemaphoreCreateInfo {
        ..Default::default()
    };

    unsafe { device.create_semaphore(&create_info, None) }.expect("Failed to create semaphore")
}
