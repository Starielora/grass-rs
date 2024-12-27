use std::ffi::c_void;
use std::{
    borrow::Cow,
    error::Error,
    ffi::{CStr, CString},
    mem::{self, size_of},
};

use ash::vk::Handle;
use ash::{
    ext::debug_utils,
    khr::{surface, swapchain, win32_surface},
    vk::{
        DeviceMemory, Extent2D, PresentModeKHR, QueueFlags, SurfaceFormatKHR, SurfaceKHR,
        SwapchainKHR,
    },
    Instance,
};
use ash::{vk, Entry};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::bindless_descriptor_set;
use crate::camera::GPUCameraData;

#[allow(dead_code)]
pub struct PhysicalDeviceProps {
    pub props: vk::PhysicalDeviceProperties,
    pub memory_props: vk::PhysicalDeviceMemoryProperties,
}

pub struct Image {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub memory: vk::DeviceMemory,
}

#[allow(dead_code)]
pub struct Images {
    pub images: Vec<vk::Image>,
    pub views: Vec<vk::ImageView>,
}

#[allow(dead_code)]
pub struct CameraVkData {
    pub buffer: vk::Buffer,
    pub buffer_address: vk::DeviceAddress,
    pub memory: vk::DeviceMemory,
    pub allocation_size: u64,
    pub data_ptr: *mut c_void,
    pub data_slice: ash::util::Align<GPUCameraData>,
}

#[allow(dead_code)]
pub struct Context {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug_utils: ash::ext::debug_utils::Instance,
    pub debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    pub surface_khr: vk::SurfaceKHR,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue_family_index: u32,
    pub device: ash::Device,
    pub present_queue: vk::Queue,
    pub swapchain: vk::SwapchainKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub window_extent: vk::Extent2D,
    pub surface_loader: ash::khr::surface::Instance,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub physical_device_props: PhysicalDeviceProps,
    pub color_image: Image,
    pub depth_image: Image,
    pub depth_image_format: vk::Format,
    pub swapchain_images: Images,
    pub camera: CameraVkData,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub acquire_semaphore: vk::Semaphore,
    pub wait_semaphore: vk::Semaphore,
    pub physical_device_memory_props: vk::PhysicalDeviceMemoryProperties,
    pub descriptor_pool: vk::DescriptorPool, // TODO this has to yeet from here
    pub descriptor_set: vk::DescriptorSet,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{message_severity:?}: {message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

fn create_instance(entry: &Entry, debug: &mut vk::DebugUtilsMessengerCreateInfoEXT) -> Instance {
    let app_name = CString::new("app name").unwrap();
    let engine_name = CString::new("engine name").unwrap();
    let app_info = vk::ApplicationInfo::default()
        .application_name(app_name.as_c_str())
        .application_version(0)
        .engine_name(engine_name.as_c_str())
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let layers_str = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    let layers = layers_str
        .iter()
        .map(|str| str.as_ptr())
        .collect::<Vec<_>>();

    let extensions = [
        debug_utils::NAME.as_ptr(),
        surface::NAME.as_ptr(),
        win32_surface::NAME.as_ptr(),
    ];

    let create_info = vk::InstanceCreateInfo::default()
        .push_next(debug)
        .application_info(&app_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions);

    unsafe { entry.create_instance(&create_info, None).expect("msg") }
}

fn create_swapchain(
    window: &winit::window::Window,
    entry: &Entry,
    logical_device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    instance: &ash::Instance,
    surface: SurfaceKHR,
    queue_family_index: u32,
) -> (
    SwapchainKHR,
    SurfaceFormatKHR,
    Extent2D,
    surface::Instance,
    swapchain::Device,
) {
    let surface_loader = surface::Instance::new(&entry, instance);
    let surface_caps = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
    }
    .expect("Could not get surface caps.");
    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }
            .expect("Could not fet surface formats.");
    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    }
    .expect("Could not get present modes.");

    let chosen_extent = vk::Extent2D::default()
        .width(window.inner_size().width.clamp(
            surface_caps.min_image_extent.width,
            surface_caps.max_image_extent.width,
        ))
        .height(window.inner_size().height.clamp(
            surface_caps.min_image_extent.height,
            surface_caps.max_image_extent.height,
        ));

    let chosen_present_mode = present_modes
        .iter()
        .find(|&&mode| mode == PresentModeKHR::MAILBOX)
        .cloned()
        .unwrap_or(PresentModeKHR::FIFO);

    let chosen_image_format = surface_formats
        .iter()
        .find(|&&format| {
            format.format == vk::Format::B8G8R8A8_UNORM
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .cloned()
        .unwrap_or(
            surface_formats
                .first()
                .cloned()
                .expect("No surface formats to create swapchain."),
        );

    let queue_family_indices = [queue_family_index];
    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(surface_caps.min_image_count)
        .image_format(chosen_image_format.format)
        .image_color_space(chosen_image_format.color_space)
        .image_extent(chosen_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(chosen_present_mode)
        .clipped(true);

    let swapchain_loader = swapchain::Device::new(&instance, logical_device);
    (
        unsafe { swapchain_loader.create_swapchain(&create_info, None) }
            .expect("Could not create swapchain"),
        chosen_image_format,
        chosen_extent,
        surface_loader,
        swapchain_loader,
    )
}

fn find_memory_type(
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    memory_type_requirements: u32,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> u32 {
    for i in 0..memory_props.memory_type_count {
        let memory_type = memory_props.memory_types[i as usize];

        if (memory_type_requirements & (1 << i)) > 0
            && (memory_type.property_flags & memory_property_flags) == memory_property_flags
        {
            return i;
        }
    }

    panic!("Could not find memory type.");
}

fn allocate_memory(
    device: &ash::Device,
    memory_requirements: &vk::MemoryRequirements,
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> DeviceMemory {
    let memory_type_index = find_memory_type(
        memory_props,
        memory_requirements.memory_type_bits,
        memory_property_flags,
    );

    let allocate_info = vk::MemoryAllocateInfo {
        allocation_size: memory_requirements.size,
        memory_type_index,
        ..Default::default()
    };

    unsafe { device.allocate_memory(&allocate_info, None) }.expect("Could not allocate memory")
}

fn create_color_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
) -> vk::ImageView {
    let create_info = vk::ImageViewCreateInfo {
        image,
        view_type: vk::ImageViewType::TYPE_2D,
        format,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };

    unsafe { device.create_image_view(&create_info, None) }.expect("Could not create image")
}

fn create_color_image(
    device: &ash::Device,
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    format: vk::Format,
    extent: &vk::Extent2D,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> (vk::Image, vk::ImageView, vk::DeviceMemory) {
    let create_info = vk::ImageCreateInfo {
        flags: vk::ImageCreateFlags::empty(),
        image_type: vk::ImageType::TYPE_2D,
        format,
        extent: vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        ..Default::default()
    };

    let image =
        unsafe { device.create_image(&create_info, None) }.expect("Could not create color image");
    let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory = allocate_memory(
        device,
        &memory_requirements,
        &memory_props,
        memory_property_flags,
    );

    unsafe { device.bind_image_memory(image, memory, 0) }.expect("Could not bind memory to image");

    let view = create_color_image_view(device, image, format);

    return (image, view, memory);
}

fn create_depth_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView, Box<dyn Error>> {
    let create_info = vk::ImageViewCreateInfo {
        image,
        view_type: vk::ImageViewType::TYPE_2D,
        format,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };

    Ok(unsafe { device.create_image_view(&create_info, None) }?)
}

fn create_depth_image(
    device: &ash::Device,
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    format: vk::Format,
    extent: &vk::Extent2D,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::ImageView, vk::DeviceMemory), Box<dyn Error>> {
    let create_info = vk::ImageCreateInfo {
        image_type: vk::ImageType::TYPE_2D,
        format,
        extent: vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        ..Default::default()
    };

    let image = unsafe { device.create_image(&create_info, None) }?;
    let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory = allocate_memory(
        device,
        &memory_requirements,
        memory_props,
        memory_property_flags,
    );

    unsafe { device.bind_image_memory(image, memory, 0) }?;

    let view = create_depth_image_view(device, image, format)?;

    Ok((image, view, memory))
}

fn uniform_buffer_padded_size(
    size: u64,
    physical_device_props: &vk::PhysicalDeviceProperties,
) -> u64 {
    let min_buffer_alignment = physical_device_props
        .limits
        .min_uniform_buffer_offset_alignment;

    let mut aligned_size = size;

    if min_buffer_alignment > 0 {
        aligned_size = (aligned_size + min_buffer_alignment - 1) & !(min_buffer_alignment - 1);
    }

    aligned_size
}

fn find_memory_type_index(
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    memory_type_requirements: u32,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> u32 {
    for i in 0..memory_props.memory_type_count {
        let memory_type = memory_props.memory_types[i as usize];
        if (memory_type_requirements & (1 << i) > 0)
            && (memory_type.property_flags & memory_property_flags) == memory_property_flags
        {
            return i;
        }
    }

    panic!("Could not find memory type");
}

fn allocate_buffer_memory(
    device: &ash::Device,
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    memory_property_flags: vk::MemoryPropertyFlags,
    buffer: vk::Buffer,
    usage: vk::BufferUsageFlags,
) -> (vk::DeviceMemory, u64) {
    let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let memory_type_index = find_memory_type_index(
        memory_props,
        memory_requirements.memory_type_bits,
        memory_property_flags,
    );

    let mut allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(memory_requirements.size)
        .memory_type_index(memory_type_index);

    // TODO this situation looks like it could be done better
    let mut device_address_allocate_flags =
        vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);

    if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
        allocate_info = allocate_info.push_next(&mut device_address_allocate_flags);
    }

    (
        unsafe { device.allocate_memory(&allocate_info, None).unwrap() },
        memory_requirements.size,
    )
}

fn create_buffer(
    device: &ash::Device,
    size: u64,
    usage: vk::BufferUsageFlags,
    memory_property_flags: vk::MemoryPropertyFlags,
    physical_device_props: &vk::PhysicalDeviceProperties,
    memory_props: &vk::PhysicalDeviceMemoryProperties,
) -> (vk::Buffer, vk::DeviceMemory, u64) {
    let aligned_size = uniform_buffer_padded_size(size, physical_device_props);

    let create_info = vk::BufferCreateInfo {
        flags: vk::BufferCreateFlags::empty(),
        size: aligned_size,
        usage,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };

    let buffer = unsafe { device.create_buffer(&create_info, None).unwrap() };

    let (memory, allocation_size) =
        allocate_buffer_memory(device, memory_props, memory_property_flags, buffer, usage);

    unsafe { device.bind_buffer_memory(buffer, memory, 0).unwrap() };

    (buffer, memory, allocation_size)
}

impl Context {
    pub fn new(window: &winit::window::Window) -> Context {
        let entry = unsafe { Entry::load().expect("Could not find Vulkan.") };

        let mut debug_utils_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(debug_callback));

        let instance = create_instance(&entry, &mut debug_utils_messenger_create_info);

        let debug_utils = debug_utils::Instance::new(&entry, &instance);
        let debug_utils_messenger = unsafe {
            debug_utils
                .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)
                .expect("Could not create debug utils messenger")
        };

        let surface_khr = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                Option::None,
            )
            .unwrap()
        };

        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Could not enumerate physical devices.")
        };

        let (graphics_queue_family_index, physical_device) = physical_devices
            .iter()
            .find_map(|device| {
                let queue_family_props =
                    unsafe { instance.get_physical_device_queue_family_properties(*device) };
                queue_family_props
                    .iter()
                    .enumerate()
                    .find_map(|(queue_family_index, props)| {
                        let suitable = props.queue_flags.contains(QueueFlags::GRAPHICS);
                        if suitable {
                            Some((queue_family_index as u32, *device))
                        } else {
                            None
                        }
                    })
            })
            .expect("Could not find suitable physical device.");

        let queue_prios = [1.0];
        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(&queue_prios)];
        let device_extensions = [swapchain::NAME.as_ptr()];

        let mut vk12_physical_device_features = vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            // bindless
            .runtime_descriptor_array(true)
            .descriptor_binding_partially_bound(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .descriptor_binding_sampled_image_update_after_bind(true);

        let mut vk13_physical_device_features =
            vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);

        let logical_device_create_info = vk::DeviceCreateInfo::default()
            .push_next(&mut vk12_physical_device_features)
            .push_next(&mut vk13_physical_device_features)
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions);

        let device =
            unsafe { instance.create_device(physical_device, &logical_device_create_info, None) }
                .expect("Could not create logical device");

        let (descriptor_pool, descriptor_set_layout, descriptor_set) =
            bindless_descriptor_set::create(&device);

        let present_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let (swapchain, surface_format, window_extent, surface_loader, swapchain_loader) =
            create_swapchain(
                &window,
                &entry,
                &device,
                physical_device,
                &instance,
                surface_khr,
                graphics_queue_family_index,
            );

        let memory_props =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let physical_device_props =
            unsafe { instance.get_physical_device_properties(physical_device) };

        let (color_image, color_image_view, color_image_memory) = create_color_image(
            &device,
            &memory_props,
            surface_format.format,
            &window_extent,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let depth_format = vk::Format::D32_SFLOAT; // todo query this from device
        let (depth_image, depth_image_view, depth_image_memory) = create_depth_image(
            &device,
            &memory_props,
            depth_format,
            &window_extent,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .unwrap();

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }
            .expect("Could not get swapchain images");
        let swapchain_images_views = swapchain_images
            .iter()
            .map(|image| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
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
                    .expect("Could not create swapchain image view")
            })
            .collect::<Vec<_>>();

        let (camera_data_buffer, camera_data_memory, camera_data_allocation_size) = create_buffer(
            &device,
            size_of::<GPUCameraData>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &physical_device_props,
            &memory_props,
        );

        let buffer_address_info = vk::BufferDeviceAddressInfo {
            buffer: camera_data_buffer,
            ..Default::default()
        };

        let camera_buffer_address =
            unsafe { device.get_buffer_device_address(&buffer_address_info) };

        let camera_data_ptr = unsafe {
            device
                .map_memory(
                    camera_data_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap()
        };
        let camera_data_slice = unsafe {
            ash::util::Align::new(
                camera_data_ptr,
                mem::align_of::<GPUCameraData>() as u64,
                camera_data_allocation_size,
            )
        };

        let command_pool_create_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: graphics_queue_family_index,
            ..Default::default()
        };

        let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }
            .expect("Could not create command pool");

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };

        let command_buffers =
            unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }
                .expect("Could not allocate command buffer");

        let semaphore_create_info = vk::SemaphoreCreateInfo {
            ..Default::default()
        };

        let acquire_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }
            .expect("Could not create semaphore");
        let wait_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }
            .expect("Could not create semaphore");

        Self {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
            surface_khr,
            physical_device,
            graphics_queue_family_index,
            device,
            present_queue,
            swapchain,
            surface_format,
            window_extent,
            surface_loader,
            swapchain_loader,
            physical_device_props: PhysicalDeviceProps {
                props: physical_device_props,
                memory_props,
            },
            color_image: Image {
                image: color_image,
                view: color_image_view,
                memory: color_image_memory,
            },
            depth_image: Image {
                image: depth_image,
                view: depth_image_view,
                memory: depth_image_memory,
            },
            depth_image_format: depth_format,
            swapchain_images: Images {
                images: swapchain_images,
                views: swapchain_images_views,
            },
            camera: CameraVkData {
                buffer: camera_data_buffer,
                buffer_address: camera_buffer_address,
                memory: camera_data_memory,
                allocation_size: camera_data_allocation_size,
                data_ptr: camera_data_ptr,
                data_slice: camera_data_slice,
            },
            command_pool,
            command_buffers,
            acquire_semaphore,
            wait_semaphore,
            physical_device_memory_props: memory_props,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
        }
    }

    pub fn create_buffer(
        self: &Self,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_propery_flags: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory, u64) {
        create_buffer(
            &self.device,
            size,
            usage,
            memory_propery_flags,
            &self.physical_device_props.props,
            &self.physical_device_props.memory_props,
        )
    }

    pub fn image_barrier(
        self: &Self,
        command_buffer: vk::CommandBuffer,
        image: vk::Image,
        old_image_layout: vk::ImageLayout,
        new_image_layout: vk::ImageLayout,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        subresource_range: vk::ImageSubresourceRange,
    ) {
        let mut memory_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_image_layout)
            .new_layout(new_image_layout)
            .image(image)
            .subresource_range(subresource_range);

        match old_image_layout {
            vk::ImageLayout::UNDEFINED => memory_barrier.src_access_mask = vk::AccessFlags::NONE,
            vk::ImageLayout::PREINITIALIZED => {
                memory_barrier.src_access_mask = vk::AccessFlags::HOST_WRITE
            }
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
                memory_barrier.src_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            }
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                memory_barrier.src_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
            }
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => {
                memory_barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ
            }
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => {
                memory_barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE
            }
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
                memory_barrier.src_access_mask = vk::AccessFlags::SHADER_READ
            }
            vk::ImageLayout::PRESENT_SRC_KHR => {
                memory_barrier.src_access_mask = vk::AccessFlags::empty();
            }
            _ => todo!("TBD"),
        }

        match new_image_layout {
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => {
                memory_barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE
            }
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => {
                memory_barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ
            }
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
                memory_barrier.dst_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            }
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                memory_barrier.dst_access_mask =
                    memory_barrier.dst_access_mask | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
            }
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
                if memory_barrier.src_access_mask == vk::AccessFlags::NONE {
                    memory_barrier.src_access_mask =
                        vk::AccessFlags::HOST_WRITE | vk::AccessFlags::TRANSFER_WRITE;
                }
                memory_barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
            }
            vk::ImageLayout::PRESENT_SRC_KHR => {
                memory_barrier.dst_access_mask = vk::AccessFlags::empty();
            }
            _ => todo!("TBD"),
        }

        let mem_barriers = [];
        let buffer_barriers = [];
        let image_barriers = [memory_barrier];

        unsafe {
            self.device.cmd_pipeline_barrier(
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

    pub fn allocage_image_memory(self: &Self, image: vk::Image) -> vk::DeviceMemory {
        let image_mem_reqs = unsafe { self.device.get_image_memory_requirements(image) };
        let image_mem_type = find_memory_type(
            &self.physical_device_memory_props,
            image_mem_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let mem_alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(image_mem_reqs.size)
            .memory_type_index(image_mem_type);
        let image_mem = unsafe {
            self.device
                .allocate_memory(&mem_alloc_info, None)
                .expect("Failed to allocate skybox image memory")
        };
        unsafe {
            self.device
                .bind_image_memory(image, image_mem, 0)
                .expect("Failed to bind skybox image memory")
        };

        image_mem
    }

    pub fn create_command_buffer(
        self: &Self,
        level: vk::CommandBufferLevel,
        begin: bool,
    ) -> vk::CommandBuffer {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .level(level)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let cmd_buffer = unsafe {
            self.device
                .allocate_command_buffers(&allocate_info)
                .expect("Failed to allocate command buffer.")
        }[0];

        if begin {
            let begin_info = vk::CommandBufferBeginInfo::default();
            unsafe {
                self.device
                    .begin_command_buffer(cmd_buffer, &begin_info)
                    .expect("Failed to begin command buffer")
            };
        }

        cmd_buffer
    }

    pub fn flush_command_buffer(self: &Self, cmd_buffer: vk::CommandBuffer, free: bool) {
        if cmd_buffer.is_null() {
            return;
        }

        unsafe {
            self.device
                .end_command_buffer(cmd_buffer)
                .expect("Faild to end command buffer")
        };

        let cmd_buffers = [cmd_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&cmd_buffers);
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::empty());

        unsafe {
            let fence = self
                .device
                .create_fence(&fence_info, None)
                .expect("Failed to create fence");

            let submits = [submit_info];

            self.device
                .queue_submit(self.present_queue, &submits, fence)
                .expect("Failed to submit queue");

            let fences = [fence];
            self.device
                .wait_for_fences(&fences, true, 10000000000)
                .expect("Error waiting for fences");
            self.device.destroy_fence(fence, None);
        };

        if free {
            unsafe {
                let buffers = [cmd_buffer];
                self.device
                    .free_command_buffers(self.command_pool, &buffers);
            }
        }
    }
}

impl std::ops::Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.wait_semaphore, None);
            self.device.destroy_semaphore(self.acquire_semaphore, None);
            let command_buffers_to_free = [*self.command_buffers.first().unwrap()];
            self.device
                .free_command_buffers(self.command_pool, &command_buffers_to_free);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.free_memory(self.camera.memory, None);
            self.device.destroy_buffer(self.camera.buffer, None);
            self.device.free_memory(self.depth_image.memory, None);
            self.device.destroy_image_view(self.depth_image.view, None);
            self.device.destroy_image(self.depth_image.image, None);
            self.device.free_memory(self.color_image.memory, None);
            self.device.destroy_image_view(self.color_image.view, None);
            self.device.destroy_image(self.color_image.image, None);
            self.swapchain_images.views.iter().for_each(|iv| {
                self.device.destroy_image_view(*iv, None);
            });
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface_khr, None);
            self.device.destroy_device(None);
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}
