pub mod buffer;
pub mod command_pool;
pub mod debug_utils;
pub mod descriptor_set;
pub mod device;
pub mod device_memory;
pub mod device_queue;
pub mod fence;
pub mod image;
pub mod instance;
pub mod physical_device;
pub mod semaphore;
pub mod swapchain;
pub mod vk_destroy;

use ash::vk;

pub fn image_barrier(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    src: (vk::ImageLayout, vk::AccessFlags, vk::PipelineStageFlags),
    dst: (vk::ImageLayout, vk::AccessFlags, vk::PipelineStageFlags),
    subresource_range: vk::ImageSubresourceRange,
) {
    let (src_image_layout, src_access_mask, src_stage_mask) = src;
    let (dst_image_layout, dst_access_mask, dst_stage_mask) = dst;

    let memory_barrier = vk::ImageMemoryBarrier::default()
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask)
        .old_layout(src_image_layout)
        .new_layout(dst_image_layout)
        .image(image)
        .subresource_range(subresource_range);

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
