use std::ffi::c_void;
use std::{
    borrow::Cow,
    error::Error,
    ffi::{CStr, CString},
    mem::{self, size_of},
};

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

use crate::camera::CameraData;

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
pub struct Pipeline {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub descriptor_set: vk::DescriptorSet,
}
#[allow(dead_code)]
pub struct CameraVkData {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub allocation_size: u64,
    pub data_ptr: *mut c_void,
    pub data_slice: ash::util::Align<CameraData>,
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
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub descriptor_pool: vk::DescriptorPool,
    pub graphics_pipeline: Pipeline,
    pub camera: CameraVkData,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub acquire_semaphore: vk::Semaphore,
    pub wait_semaphore: vk::Semaphore,
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
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
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

fn create_render_pass(
    logical_device: &ash::Device,
    swapchain_format: vk::Format,
    depth_format: vk::Format,
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

    let depth_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: depth_format,
        samples: vk::SampleCountFlags::TYPE_8,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::CLEAR,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
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

    let depth_attachment_reference = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let resolve_attachment_reference = vk::AttachmentReference {
        attachment: 2,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let color_attachment_references = [color_attachment_reference];
    let resolve_attachment_reference = [resolve_attachment_reference];
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_references)
        .depth_stencil_attachment(&depth_attachment_reference)
        .resolve_attachments(&resolve_attachment_reference);

    let subpass_dependencies = [
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        },
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        },
    ];

    let attachments = [color_attachment, depth_attachment, color_resolve_attachment];
    let subpasses = [subpass];
    let render_pass_create_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&subpass_dependencies);

    let render_pass = unsafe { logical_device.create_render_pass(&render_pass_create_info, None) }
        .expect("Could not create render pass");

    render_pass
}

fn create_framebuffers(
    logical_device: &ash::Device,
    render_pass: &vk::RenderPass,
    color_image_view: &vk::ImageView,
    depth_image_view: &vk::ImageView,
    swapchain_image_views: &[vk::ImageView],
    extent: &vk::Extent2D,
) -> Vec<vk::Framebuffer> {
    swapchain_image_views
        .iter()
        .map(|view| {
            let attachments = [*color_image_view, *depth_image_view, *view];
            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(*render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

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
        samples: vk::SampleCountFlags::TYPE_8,
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

fn create_graphics_pipeline_layout(
    device: &ash::Device,
    layout: vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let layouts = [layout];
    let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts);
    unsafe { device.create_pipeline_layout(&create_info, None).unwrap() }
}

fn create_graphics_pipeline(
    device: &ash::Device,
    window_extent: &vk::Extent2D,
    pipeline_layout: &vk::PipelineLayout,
    render_pass: &vk::RenderPass,
) -> vk::Pipeline {
    // todo path lol
    let mut vs_spv_file = std::fs::File::open("target/debug/triangle.vert.spv").unwrap();
    let vs_spv = ash::util::read_spv(&mut vs_spv_file).unwrap();
    let vs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&vs_spv);
    let vs_module = unsafe {
        device
            .create_shader_module(&vs_shader_module_create_info, None)
            .unwrap()
    };
    let shader_main = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let mut fs_spv_file = std::fs::File::open("target/debug/triangle.frag.spv").unwrap();
    let fs_spv = ash::util::read_spv(&mut fs_spv_file).unwrap();
    let fs_shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&fs_spv);
    let fs_module = unsafe {
        device
            .create_shader_module(&fs_shader_module_create_info, None)
            .unwrap()
    };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vs_module,
            p_name: shader_main.as_ptr(),
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: fs_module,
            p_name: shader_main.as_ptr(),
            ..Default::default()
        },
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
        ..Default::default()
    };

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_STRIP,
        ..Default::default()
    };

    let viewport = vk::Viewport {
        width: window_extent.width as f32,
        height: window_extent.height as f32,
        max_depth: 1.0,
        ..Default::default()
    };

    let scissors = vk::Rect2D {
        extent: *window_extent,
        ..Default::default()
    };

    let viewports = [viewport];
    let scissors = [scissors];
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::CLOCKWISE,
        depth_bias_enable: vk::FALSE,
        line_width: 1.0,
        ..Default::default()
    };

    let multisample_state = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_8,
        sample_shading_enable: vk::FALSE,
        min_sample_shading: 1.0,
        alpha_to_coverage_enable: vk::FALSE,
        alpha_to_one_enable: vk::FALSE,
        ..Default::default()
    };

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: vk::TRUE,
        depth_write_enable: vk::TRUE,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        ..Default::default()
    };

    let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::TRUE,
        src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    };

    let attachments = [color_blend_attachment_state];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    let create_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(*pipeline_layout)
        .render_pass(*render_pass);

    let pipelines = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .unwrap()
    };

    unsafe {
        device.destroy_shader_module(vs_module, None);
        device.destroy_shader_module(fs_module, None);
    }

    pipelines[0]
}

fn create_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let bindings = [vk::DescriptorSetLayoutBinding {
        binding: 0u32,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
        _marker: std::marker::PhantomData,
    }];

    let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    unsafe {
        device
            .create_descriptor_set_layout(&create_info, None)
            .unwrap()
    }
}

fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
    let create_info = vk::DescriptorPoolCreateInfo::default()
        .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
        .max_sets(16)
        .pool_sizes(&[vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        }]);

    unsafe { device.create_descriptor_pool(&create_info, None).unwrap() }
}

fn allocate_descriptor_set(
    device: &ash::Device,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> vk::DescriptorSet {
    let layouts = [layout];
    let allocate_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(pool)
        .set_layouts(&layouts);

    let set = unsafe { device.allocate_descriptor_sets(&allocate_info).unwrap() };

    set[0]
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
) -> (vk::DeviceMemory, u64) {
    let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let memory_type_index = find_memory_type_index(
        memory_props,
        memory_requirements.memory_type_bits,
        memory_property_flags,
    );

    let allocate_info = vk::MemoryAllocateInfo {
        allocation_size: memory_requirements.size,
        memory_type_index,
        ..Default::default()
    };

    (
        unsafe { device.allocate_memory(&allocate_info, None).unwrap() },
        memory_requirements.size,
    )
}

fn create_uniform_buffer(
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
        allocate_buffer_memory(device, memory_props, memory_property_flags, buffer);

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

        let logical_device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions);

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

        let render_pass = create_render_pass(&device, surface_format.format, depth_format);
        let framebuffers = create_framebuffers(
            &device,
            &render_pass,
            &color_image_view,
            &depth_image_view,
            &swapchain_images_views,
            &window_extent,
        );

        let graphics_pipeline_descriptor_set_layout = create_descriptor_set_layout(&device);
        let graphics_pipeline_layout =
            create_graphics_pipeline_layout(&device, graphics_pipeline_descriptor_set_layout);
        let graphics_pipeline = create_graphics_pipeline(
            &device,
            &window_extent,
            &graphics_pipeline_layout,
            &render_pass,
        );

        let descriptor_pool = create_descriptor_pool(&device);
        let graphics_pipeline_descriptor_set = allocate_descriptor_set(
            &device,
            descriptor_pool,
            graphics_pipeline_descriptor_set_layout,
        );

        let (camera_data_buffer, camera_data_memory, camera_data_allocation_size) =
            create_uniform_buffer(
                &device,
                size_of::<CameraData>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &physical_device_props,
                &memory_props,
            );
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
                mem::align_of::<CameraData>() as u64,
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
            render_pass,
            framebuffers,
            descriptor_pool,
            graphics_pipeline: Pipeline {
                descriptor_set_layout: graphics_pipeline_descriptor_set_layout,
                pipeline_layout: graphics_pipeline_layout,
                pipeline: graphics_pipeline,
                descriptor_set: graphics_pipeline_descriptor_set,
            },
            camera: CameraVkData {
                buffer: camera_data_buffer,
                memory: camera_data_memory,
                allocation_size: camera_data_allocation_size,
                data_ptr: camera_data_ptr,
                data_slice: camera_data_slice,
            },
            command_pool,
            command_buffers,
            acquire_semaphore,
            wait_semaphore,
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
            self.device.free_memory(self.camera.memory, None);
            self.device.destroy_buffer(self.camera.buffer, None);
            self.device
                .destroy_descriptor_set_layout(self.graphics_pipeline.descriptor_set_layout, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_pipeline_layout(self.graphics_pipeline.pipeline_layout, None);
            self.device
                .destroy_pipeline(self.graphics_pipeline.pipeline, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.free_memory(self.depth_image.memory, None);
            self.device.destroy_image_view(self.depth_image.view, None);
            self.device.destroy_image(self.depth_image.image, None);
            self.device.free_memory(self.color_image.memory, None);
            self.device.destroy_image_view(self.color_image.view, None);
            self.device.destroy_image(self.color_image.image, None);
            self.framebuffers.iter().for_each(|fb| {
                self.device.destroy_framebuffer(*fb, None);
            });
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
