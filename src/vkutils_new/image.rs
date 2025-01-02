use super::{device_memory, vk_destroy};
use ash::vk;

pub struct Image {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub memory: vk::DeviceMemory,
    pub format: vk::Format,
    device: ash::Device,
}

impl Image {
    // assume 2D images
    pub fn new(
        device: ash::Device,
        flags: vk::ImageCreateFlags,
        format: vk::Format,
        extent: vk::Extent2D,
        array_layers: u32,
        samples: vk::SampleCountFlags,
        usage: vk::ImageUsageFlags,
        aspect_flags: vk::ImageAspectFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
        memory_props: &vk::PhysicalDeviceMemoryProperties,
    ) -> Self {
        let create_info = vk::ImageCreateInfo {
            flags,
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers,
            samples,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED, // must be UNDEFINED or PREINITIALIZED
            ..Default::default()
        };

        let image =
            unsafe { device.create_image(&create_info, None) }.expect("Failed to create image");

        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };

        let memory = device_memory::allocate(
            &device,
            &memory_requirements,
            &memory_props,
            memory_property_flags,
            false,
        );

        unsafe { device.bind_image_memory(image, memory, 0) }
            .expect("Failed to bind memory to image");

        let view = create_image_view(&device, image, format, aspect_flags, array_layers);

        Self {
            handle: image,
            view,
            memory,
            format,
            device,
        }
    }
}

fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    aspect_mask: vk::ImageAspectFlags,
    layer_count: u32,
) -> vk::ImageView {
    let create_info = vk::ImageViewCreateInfo {
        image,
        view_type: match layer_count {
            1 => vk::ImageViewType::TYPE_2D,
            6 => vk::ImageViewType::CUBE,
            _ => todo!(),
        },
        format,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count,
        },
        ..Default::default()
    };

    unsafe { device.create_image_view(&create_info, None) }.expect("Failed to create image view.")
}

impl vk_destroy::VkDestroy for Image {
    fn vk_destroy(&self) {
        unsafe {
            self.device.free_memory(self.memory, None);
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.handle, None);
        }
    }
}
