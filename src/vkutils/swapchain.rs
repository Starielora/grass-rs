use super::vk_destroy;
use ash::vk;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

const PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::FIFO;

pub struct Swapchain {
    device: ash::Device,
    surface: vk::SurfaceKHR,
    pub swapchain: vk::SwapchainKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    surface_caps: vk::SurfaceCapabilitiesKHR,
    pub extent: vk::Extent2D,
    surface_instance: ash::khr::surface::Instance,
    swapchain_device: ash::khr::swapchain::Device,
    pub images: std::vec::Vec<vk::Image>,
    pub views: std::vec::Vec<vk::ImageView>,
    queue_family_index: u32,
    pub acquire_semaphore: vk::Semaphore,
    physical_device: vk::PhysicalDevice,
}

impl Swapchain {
    pub fn new(
        window: &winit::window::Window,
        entry: &ash::Entry,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
        queue_family_index: u32,
    ) -> Self {
        let (surface, surface_instance, surface_caps, surface_formats, _present_modes) =
            init_surface(&entry, &instance, &window, physical_device);

        let extent = get_extent(&window, surface_caps);
        let present_mode = PRESENT_MODE;
        let surface_format = get_surface_format(surface_formats);
        let queue_family_indices = [queue_family_index];

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(surface_caps.min_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain_device = ash::khr::swapchain::Device::new(&instance, device);
        let swapchain = unsafe { swapchain_device.create_swapchain(&create_info, None) }
            .expect("Failed to create swapchain");

        let (images, views) =
            get_images(&device, &swapchain_device, swapchain, surface_format.format);

        let acquire_semaphore = unsafe {
            let create_info = vk::SemaphoreCreateInfo::default();
            device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore")
        };

        Self {
            device: device.clone(),
            surface,
            swapchain,
            surface_format,
            surface_caps,
            extent,
            surface_instance,
            swapchain_device,
            images,
            views,
            queue_family_index,
            acquire_semaphore,
            physical_device,
        }
    }

    // pub fn acquire_next_image(&self, timeout: u64, fence: vk::Fence) -> (u32, vk::Semaphore, bool) {
    //     // TODO expect here will throw on VK_ERROR_OUT_OF_DATE, which prevents proper handling
    //     let (image_index, is_suboptimal) = unsafe {
    //         self.swapchain_device
    //             .acquire_next_image(self.swapchain, timeout, self.acquire_semaphore, fence)
    //             .expect("Failed to acquire next swapchain image.")
    //     };

    //     (image_index, self.acquire_semaphore, is_suboptimal)
    // }

    pub fn acquire_next_image(
        &self,
        timeout: u64,
        fence: vk::Fence,
    ) -> (ash::prelude::VkResult<(u32, bool)>, vk::Semaphore) {
        let result = unsafe {
            self.swapchain_device.acquire_next_image(
                self.swapchain,
                timeout,
                self.acquire_semaphore,
                fence,
            )
        };

        (result, self.acquire_semaphore)
    }

    pub fn present(
        &self,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore],
        queue: vk::Queue,
    ) -> ash::prelude::VkResult<bool> {
        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .wait_semaphores(&wait_semaphores)
            .image_indices(&image_indices);

        // TODO expect here will throw on VK_ERROR_OUT_OF_DATE, which prevents proper handling
        unsafe { self.swapchain_device.queue_present(queue, &present_info) }
    }

    // TODO most likely this is just a resize, not rebuild. For rebuild surface props should be queried.
    // Rebuild should be issued on VK_ERROR_OUT_OF_DATE, while resize on window resize or suboptimal flag returned from present or acquire next image
    // Anyway, calling this fn with zero extent may fail
    pub fn rebuild(&mut self, window: &winit::window::Window) {
        let present_mode = PRESENT_MODE;
        let surface_format = self.surface_format;
        let queue_family_indices = [self.queue_family_index];

        self.surface_caps = unsafe {
            self.surface_instance
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
        }
        .expect("Failed to get surface caps");
        let extent = get_extent(&window, self.surface_caps);

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(self.surface_caps.min_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(self.surface_caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(self.swapchain);

        let swapchain = unsafe { self.swapchain_device.create_swapchain(&create_info, None) }
            .expect("Failed to create swapchain");

        let (images, views) = get_images(
            &self.device,
            &self.swapchain_device,
            swapchain,
            surface_format.format,
        );

        unsafe {
            self.device.device_wait_idle().expect("Failed to wait idle");
            self.views.iter().for_each(|iv| {
                self.device.destroy_image_view(*iv, None);
            });
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
        }

        self.swapchain = swapchain;
        self.images = images;
        self.views = views;
        self.extent = extent;
    }
}

impl vk_destroy::VkDestroy for Swapchain {
    fn vk_destroy(&self) {
        unsafe {
            self.views.iter().for_each(|iv| {
                self.device.destroy_image_view(*iv, None);
            });
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.surface_instance.destroy_surface(self.surface, None);
            self.device.destroy_semaphore(self.acquire_semaphore, None);
        }
    }
}

fn init_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
    physical_device: vk::PhysicalDevice,
) -> (
    vk::SurfaceKHR,
    ash::khr::surface::Instance,
    vk::SurfaceCapabilitiesKHR,
    std::vec::Vec<vk::SurfaceFormatKHR>,
    std::vec::Vec<vk::PresentModeKHR>,
) {
    let surface = unsafe {
        ash_window::create_surface(
            &entry,
            &instance,
            window.display_handle().unwrap().as_raw(),
            window.window_handle().unwrap().as_raw(),
            Option::None,
        )
        .expect("Failed to create surface")
    };

    let surface_instance = ash::khr::surface::Instance::new(&entry, instance);

    let surface_caps = unsafe {
        surface_instance.get_physical_device_surface_capabilities(physical_device, surface)
    }
    .expect("Failed to get surface caps.");

    let surface_formats =
        unsafe { surface_instance.get_physical_device_surface_formats(physical_device, surface) }
            .expect("Failed to get surface formats.");

    let present_modes = unsafe {
        surface_instance.get_physical_device_surface_present_modes(physical_device, surface)
    }
    .expect("Failed to get present modes.");

    (
        surface,
        surface_instance,
        surface_caps,
        surface_formats,
        present_modes,
    )
}

fn get_extent(
    window: &winit::window::Window,
    surface_caps: vk::SurfaceCapabilitiesKHR,
) -> vk::Extent2D {
    vk::Extent2D::default()
        .width(window.inner_size().width.clamp(
            surface_caps.min_image_extent.width,
            surface_caps.max_image_extent.width,
        ))
        .height(window.inner_size().height.clamp(
            surface_caps.min_image_extent.height,
            surface_caps.max_image_extent.height,
        ))
}

fn get_surface_format(
    surface_formats: std::vec::Vec<vk::SurfaceFormatKHR>,
) -> vk::SurfaceFormatKHR {
    let fallback_image_format = surface_formats
        .first()
        .expect("Empty surface formats collection.")
        .clone();

    surface_formats
        .into_iter()
        .find(|&format| {
            format.format == vk::Format::B8G8R8A8_UNORM
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(fallback_image_format)
}

fn get_images(
    device: &ash::Device,
    swapchain_device: &ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    format: vk::Format,
) -> (std::vec::Vec<vk::Image>, std::vec::Vec<vk::ImageView>) {
    let images = unsafe { swapchain_device.get_swapchain_images(swapchain) }
        .expect("Failed to get swapchain images");

    let views = images
        .iter()
        .map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(
                    vk::ComponentMapping::default()
                        .r(vk::ComponentSwizzle::IDENTITY)
                        .g(vk::ComponentSwizzle::IDENTITY)
                        .b(vk::ComponentSwizzle::IDENTITY)
                        .a(vk::ComponentSwizzle::IDENTITY),
                )
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );
            unsafe { device.create_image_view(&create_info, None) }
                .expect("Failed to create swapchain image view")
        })
        .collect::<Vec<_>>();

    (images, views)
}
