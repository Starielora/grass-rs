use std::{
    borrow::Cow,
    ffi::{CStr, CString},
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    platform::scancode::PhysicalKeyExtScancode,
    window::WindowBuilder,
};

use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
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

fn main() {
    let event_loop = EventLoop::new().expect("Error creating event loop.");
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Error building window.");

    let entry = unsafe { Entry::load().expect("Could not find Vulkan.") };

    // TODO how do I make it work with default?
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

    let extensions =[DebugUtils::name().as_ptr()];

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

    let create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_utils_messenger_create_info)
        .application_info(&app_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions);

    let instance = unsafe { entry.create_instance(&create_info, None).expect("msg") };

    let debug_utils = DebugUtils::new(&entry, &instance);
    let debug_utils_messenger = unsafe { debug_utils.create_debug_utils_messenger(&debug_utils_messenger_create_info, None).expect("Could not create debug utils messenger") };

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
        Event::AboutToWait => {}
        _ => (),
    });
}
