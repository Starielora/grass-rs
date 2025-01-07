use super::debug_utils;
use ash::vk;

pub fn create(entry: &ash::Entry) -> ash::Instance {
    let app_name = std::ffi::CString::new("app name").unwrap();
    let engine_name = std::ffi::CString::new("engine name").unwrap();
    let app_info = vk::ApplicationInfo::default()
        .application_name(app_name.as_c_str())
        .application_version(0)
        .engine_name(engine_name.as_c_str())
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let layers_str = [std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    let layers = layers_str
        .iter()
        .map(|str| str.as_ptr())
        .collect::<Vec<_>>();

    let extensions = [
        ash::ext::debug_utils::NAME.as_ptr(),
        ash::khr::surface::NAME.as_ptr(),
        ash::khr::win32_surface::NAME.as_ptr(),
    ];

    let mut debug = debug_utils::get_debug_utils_messenger_create_info();

    let create_info = vk::InstanceCreateInfo::default()
        .push_next(&mut debug)
        .application_info(&app_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions);

    unsafe { entry.create_instance(&create_info, None).expect("msg") }
}
