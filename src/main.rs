use std::{
    borrow::Cow,
    ffi::{c_void, CStr, CString},
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    platform::scancode::PhysicalKeyExtScancode,
    raw_window_handle::{
        HasWindowHandle, RawWindowHandle,
    },
    window::{Window, WindowBuilder},
};

use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{self, Surface, Swapchain},
    },
    prelude::VkResult,
    vk::{
        DeviceMemory, Extent2D, PresentModeKHR, QueueFlags, SurfaceFormatKHR, SurfaceKHR, SwapchainKHR
    },
    Instance,
};
use ash::{vk, Entry};

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

// TODO I could not make work ash 0.12 and winit 0.29.10 because of raw window handle incompatibility
fn create_surface(entry: &Entry, instance: &Instance, window: &Window) -> VkResult<vk::SurfaceKHR> {
    let (hinstance, hwnd) = match window.window_handle().expect("").as_raw() {
        RawWindowHandle::Win32(window) => (window.hinstance, window.hwnd),
        _ => todo!(),
    };
    let surface_fn = khr::Win32Surface::new(entry, instance);
    let surface_desc = vk::Win32SurfaceCreateInfoKHR::builder()
        .hinstance(hinstance.unwrap().get() as *const c_void)
        .hwnd(hwnd.get() as *const c_void);
    unsafe { surface_fn.create_win32_surface(&surface_desc, None) }
}

fn create_instance(entry: &Entry, debug: &mut vk::DebugUtilsMessengerCreateInfoEXT) -> Instance {
    let app_info = vk::ApplicationInfo::builder()
        .application_name(CString::new("app name").unwrap().as_c_str())
        .application_version(0)
        .engine_name(CString::new("engine name").unwrap().as_c_str())
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0))
        .build();

    let layers_str = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    let layers = layers_str
        .iter()
        .map(|str| str.as_ptr())
        .collect::<Vec<_>>();

    // TODO I could not make work ash 0.12 and winit 0.29.10 because of raw window handle incompatibility
    let extensions = [
        DebugUtils::name().as_ptr(),
        khr::Surface::name().as_ptr(),
        khr::Win32Surface::name().as_ptr(),
    ];

    let create_info = vk::InstanceCreateInfo::builder()
        .push_next(debug)
        .application_info(&app_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions);

    unsafe { entry.create_instance(&create_info, None).expect("msg") }
}

fn create_swapchain(
    window: &Window,
    entry: &Entry,
    logical_device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    instance: &ash::Instance,
    surface: SurfaceKHR,
    queue_family_index: u32,
) -> (SwapchainKHR, SurfaceFormatKHR, Extent2D, Surface, Swapchain) {
    let surface_loader = Surface::new(&entry, instance);
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

    let chosen_extent = vk::Extent2D::builder()
        .width(window.inner_size().width.clamp(
            surface_caps.min_image_extent.width,
            surface_caps.max_image_extent.width,
        ))
        .height(window.inner_size().height.clamp(
            surface_caps.min_image_extent.height,
            surface_caps.max_image_extent.height,
        ))
        .build();

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

    let create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface)
        .min_image_count(surface_caps.min_image_count)
        .image_format(chosen_image_format.format)
        .image_color_space(chosen_image_format.color_space)
        .image_extent(chosen_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(&[queue_family_index])
        .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(chosen_present_mode)
        .clipped(true)
        .build();

    let swapchain_loader = Swapchain::new(&instance, logical_device);
    (
        unsafe { swapchain_loader.create_swapchain(&create_info, None) }
            .expect("Could not create swapchain"),
        chosen_image_format,
        chosen_extent,
        surface_loader,
        swapchain_loader,
    )
}

fn create_render_pass(
    logical_device: &ash::Device,
    swapchain_format: vk::Format,
) -> vk::RenderPass {
    let color_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: swapchain_format,
        samples: vk::SampleCountFlags::TYPE_8,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let color_resolve_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: swapchain_format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::DONT_CARE,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
    };

    let color_attachment_reference = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let resolve_attachment_reference = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&[color_attachment_reference])
        .resolve_attachments(&[resolve_attachment_reference])
        .build();

    let subpass_dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        src_access_mask: vk::AccessFlags::empty(),
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dependency_flags: vk::DependencyFlags::empty(),
    }];

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&[color_attachment, color_resolve_attachment])
        .subpasses(&[subpass])
        .dependencies(&subpass_dependencies)
        .build();

    let render_pass = unsafe { logical_device.create_render_pass(&render_pass_create_info, None) }
        .expect("Could not create render pass");

    render_pass
}

fn create_framebuffers(
    logical_device: &ash::Device,
    render_pass: &vk::RenderPass,
    color_image_view: &vk::ImageView,
    swapchain_image_views: &[vk::ImageView],
    extent: &vk::Extent2D,
) -> Vec<vk::Framebuffer> {
    swapchain_image_views
        .iter()
        .map(|view| {
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(*render_pass)
                .attachments(&[*color_image_view, *view])
                .width(extent.width)
                .height(extent.height)
                .layers(1)
                .build();

            unsafe {
                logical_device
                    .create_framebuffer(&create_info, None)
                    .expect("Could not create framebuffer")
            }
        })
        .collect::<Vec<_>>()
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

fn create_color_image_view(device: &ash::Device, image: vk::Image, format: vk::Format) -> vk::ImageView {

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
    memory_property_flags: vk::MemoryPropertyFlags
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
        samples: vk::SampleCountFlags::TYPE_8,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        ..Default::default()
    };

    let image =
        unsafe { device.create_image(&create_info, None) }.expect("Could not create color image");
    let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory = allocate_memory(device, &memory_requirements, &memory_props, memory_property_flags);

    unsafe { device.bind_image_memory(image, memory, 0) }.expect("Could not bind memory to image");

    let view = create_color_image_view(device, image, format);

    return (image, view, memory);
}

fn main() {
    let event_loop = EventLoop::new().expect("Error creating event loop.");
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Error building window.");

    let entry = unsafe { Entry::load().expect("Could not find Vulkan.") };

    let mut debug_utils_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
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
        .pfn_user_callback(Some(debug_callback))
        .build();

    let instance = create_instance(&entry, &mut debug_utils_messenger_create_info);

    let debug_utils = DebugUtils::new(&entry, &instance);
    let debug_utils_messenger = unsafe {
        debug_utils
            .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)
            .expect("Could not create debug utils messenger")
    };

    let surface = create_surface(&entry, &instance, &window).expect("Could not create surface");

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
    let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(graphics_queue_family_index)
        .queue_priorities(&queue_prios)
        .build()];
    let device_extensions = [Swapchain::name().as_ptr()];

    let logical_device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions)
        .build();

    let device =
        unsafe { instance.create_device(physical_device, &logical_device_create_info, None) }
            .expect("Could not create logical device");
    let present_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

    let (swapchain, surface_format, window_extent, surface_loader, swapchain_loader) =
        create_swapchain(
            &window,
            &entry,
            &device,
            physical_device,
            &instance,
            surface,
            graphics_queue_family_index,
        );

    let memory_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let (color_image, color_image_view, device_memory) = create_color_image(&device, &memory_props, surface_format.format, &window_extent, vk::MemoryPropertyFlags::DEVICE_LOCAL);

    let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }
        .expect("Could not get swapchain images");
    let swapchain_images_views = swapchain_images
        .iter()
        .map(|image| {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(
                    vk::ComponentMapping::builder()
                        .r(vk::ComponentSwizzle::IDENTITY)
                        .g(vk::ComponentSwizzle::IDENTITY)
                        .b(vk::ComponentSwizzle::IDENTITY)
                        .a(vk::ComponentSwizzle::IDENTITY)
                        .build(),
                )
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .build();
            unsafe { device.create_image_view(&create_info, None) }
                .expect("Could not create swapchain image view")
        })
        .collect::<Vec<_>>();

    let render_pass = create_render_pass(&device, surface_format.format);
    let framebuffers = create_framebuffers(
        &device,
        &render_pass,
        &color_image_view,
        &swapchain_images_views,
        &window_extent,
    );

    let command_pool_create_info = vk::CommandPoolCreateInfo {
        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        queue_family_index: graphics_queue_family_index,
        ..Default::default()
    };

    let command_pool = unsafe { device.create_command_pool(&command_pool_create_info, None) }
        .expect("Could not create command pool");

    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        command_pool: command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: 1,
        ..Default::default()
    };

    let command_buffers = unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }
        .expect("Could not allocate command buffer");
    let command_buffer = command_buffers.first().expect("Missing command buffer");

    let semaphore_create_info = vk::SemaphoreCreateInfo {
        ..Default::default()
    };

    let acquire_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }
        .expect("Could not create semaphore");
    let wait_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }
        .expect("Could not create semaphore");

    let _ = event_loop.run(move |event, window_target| match event {
        Event::WindowEvent {
            window_id: _,
            event: WindowEvent::CloseRequested,
        } => window_target.exit(),
        Event::WindowEvent {
            window_id: _,
            event: WindowEvent::KeyboardInput { event, .. },
        } => match event.physical_key {
            winit::keyboard::PhysicalKey::Code(KeyCode::Escape) => window_target.exit(),
            winit::keyboard::PhysicalKey::Code(key) => {
                println!("Key pressed: {}", key.to_scancode().unwrap())
            }
            winit::keyboard::PhysicalKey::Unidentified(_) => todo!(),
        },
        Event::AboutToWait => {
            let (image_index, success) = unsafe {
                swapchain_loader.acquire_next_image(
                    swapchain,
                    !0,
                    acquire_semaphore,
                    vk::Fence::null(),
                )
            }
            .expect("Could not acquire image");

            unsafe {
                device.reset_command_buffer(*command_buffer, vk::CommandBufferResetFlags::empty())
            }
            .expect("Failed to reset command buffer");

            let begin_info = vk::CommandBufferBeginInfo {
                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };

            unsafe { device.begin_command_buffer(*command_buffer, &begin_info) }
                .expect("Failed to begin command buffer");
            let clear_color = vk::ClearColorValue {
                float32: [0.5, 1., 0.5, 1.0],
            };

            let render_pass_begin = vk::RenderPassBeginInfo::builder()
                .render_pass(render_pass)
                .framebuffer(framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    extent: window_extent,
                    offset: vk::Offset2D { x: 0, y: 0 },
                })
                .clear_values(&[vk::ClearValue { color: clear_color }])
                .build();

            unsafe {
                device.cmd_begin_render_pass(
                    *command_buffer,
                    &render_pass_begin,
                    vk::SubpassContents::INLINE,
                )
            };

            unsafe { device.cmd_end_render_pass(*command_buffer) };
            unsafe { device.end_command_buffer(*command_buffer) }
                .expect("Failed to end command buffer???");

            let submits = [vk::SubmitInfo::builder()
                .wait_semaphores(&[acquire_semaphore])
                .command_buffers(&[*command_buffer])
                .signal_semaphores(&[wait_semaphore])
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .build()];
            unsafe { device.queue_submit(present_queue, &submits, vk::Fence::null()) }
                .expect("Failed to submit");

            let present_info = vk::PresentInfoKHR::builder()
                .swapchains(&[swapchain])
                .wait_semaphores(&[wait_semaphore])
                .image_indices(&[image_index])
                .build();

            unsafe { swapchain_loader.queue_present(present_queue, &present_info) }
                .expect("Failed to queue present");

            unsafe { device.device_wait_idle() }.expect("Failed to wait");
        }
        _ => (),
    });

    // unsafe { swapchain_loader.destroy_swapchain(swapchain, None) };
    unsafe { surface_loader.destroy_surface(surface, None) };
    // unsafe { device.destroy_device(None) };
    unsafe { debug_utils.destroy_debug_utils_messenger(debug_utils_messenger, None) };
    unsafe { instance.destroy_instance(None) };
}
