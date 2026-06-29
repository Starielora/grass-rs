use ash::vk;

use crate::vkutils::{self, vk_destroy::VkDestroy};

pub fn load(images: [&[u8]; 6], vk: &vkutils::context::VulkanContext) -> vkutils::image::Image {
    let (staging_buffer, width, height, single_image_size) =
        load_textures_to_staging_buffer2(images, vk);

    let format = vk::Format::R8G8B8A8_UNORM;

    let image = vkutils::image::Image::new(
        vk.device.clone(),
        vk::ImageCreateFlags::CUBE_COMPATIBLE,
        format,
        vk::Extent2D { width, height },
        6,
        vk::SampleCountFlags::TYPE_1,
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        vk::ImageAspectFlags::COLOR,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        &vk.physical_device.memory_props,
    );

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .layer_count(6);

    vk.transient_graphics_command_pool.transition_image_layout(
        vk.graphics_present_queue,
        image.handle,
        (
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
        ),
        (
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        subresource_range,
    );

    vk.transient_transfer_command_pool
        .execute_short_lived_command_buffer(vk.transfer_queue, |device, command_buffer| {
            let mut buffer_copy_regions = Vec::new();

            for face in 0..6 as u64 {
                let image_subresource_layers = vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(face as u32)
                    .layer_count(1);
                let image_extent = vk::Extent3D::default()
                    .width(width as u32)
                    .height(height as u32)
                    .depth(1);
                let copy_region = vk::BufferImageCopy::default()
                    .image_subresource(image_subresource_layers)
                    .image_extent(image_extent)
                    .buffer_offset(face * single_image_size as u64);

                buffer_copy_regions.push(copy_region);
            }

            unsafe {
                device.cmd_copy_buffer_to_image(
                    command_buffer,
                    staging_buffer.handle,
                    image.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    buffer_copy_regions.as_slice(),
                )
            };
        });

    vk.transient_graphics_command_pool.transition_image_layout(
        vk.graphics_present_queue,
        image.handle,
        (
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        subresource_range,
    );

    // cleanup
    staging_buffer.vk_destroy();

    image
}

fn load_textures_to_staging_buffer2(
    files: [&[u8]; 6],
    vk: &vkutils::context::VulkanContext,
) -> (vkutils::buffer::Buffer, u32, u32, isize) {
    let mut texture_width: i32 = 0;
    let mut texture_height: i32 = 0;

    let mut staging_buffer_offset: isize = 0;

    // these are initialized on first image
    let mut staging_buffer: Option<vkutils::buffer::Buffer> = None;
    let mut single_texture_size_in_bytes: Option<isize> = None;

    for file in files {
        // let mut f = std::fs::File::open(path).expect("file not found");

        // let mut contents = vec![];
        let mut width: i32 = 0;
        let mut height: i32 = 0;
        let mut comps: i32 = 0;
        // let _ = f.read_to_end(&mut contents);

        let img_data = unsafe {
            stb_image_rust::stbi_load_from_memory(
                file.as_ptr(),
                file.len() as i32,
                &mut width,
                &mut height,
                &mut comps,
                stb_image_rust::STBI_rgb_alpha,
            )
        };

        // allocate staging buffer on first image
        if texture_width == 0 && texture_height == 0 {
            texture_width = width;
            texture_height = height;

            single_texture_size_in_bytes =
                Some(texture_width as isize * texture_height as isize * 4); // w * h * rgba comps
            let total_size_in_bytes = single_texture_size_in_bytes.unwrap() * 6; // 6 faces
            let buffer = vk.create_buffer(
                total_size_in_bytes as usize,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            );

            staging_buffer = Some(buffer);
        } else if width != texture_width || height != texture_height {
            panic!(
                "Skybox images size mismatch. Expected {}x{}, got {}x{}",
                texture_width, texture_height, width, height
            );
        }

        unsafe {
            // TODO fix this situation
            // maybe buffer should have a function to upload at offset
            std::ptr::copy_nonoverlapping(
                img_data,
                staging_buffer
                    .as_ref()
                    .unwrap()
                    .ptr
                    .unwrap()
                    .offset(staging_buffer_offset) as *mut u8,
                single_texture_size_in_bytes.unwrap() as usize,
            );

            stb_image_rust::stbi_image_free(img_data);
        }

        staging_buffer_offset += single_texture_size_in_bytes.unwrap();
    }

    staging_buffer.as_mut().unwrap().unmap_memory();

    (
        staging_buffer.unwrap(),
        texture_width as u32,
        texture_height as u32,
        single_texture_size_in_bytes.unwrap(),
    )
}
