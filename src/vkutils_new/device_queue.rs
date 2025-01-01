use ash::vk;
pub fn get_device_queues(
    device: &ash::Device,
    queue_families: &std::vec::Vec<u32>,
) -> std::vec::Vec<vk::Queue> {
    queue_families
        .iter()
        .map(|&queue_family| {
            // always get the first queue from family
            unsafe { device.get_device_queue(queue_family, 0) }
        })
        .collect()
}
