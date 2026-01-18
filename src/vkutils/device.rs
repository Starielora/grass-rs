use ash::vk;

pub fn create(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_families: &std::vec::Vec<u32>,
) -> ash::Device {
    let queue_prios = [1.0];
    let queue_create_infos: std::vec::Vec<vk::DeviceQueueCreateInfo> = queue_families
        .iter()
        .map(|&index| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(index)
                .queue_priorities(&queue_prios)
        })
        .collect();
    let device_extensions = [
        ash::khr::swapchain::NAME.as_ptr(),
        ash::khr::spirv_1_4::NAME.as_ptr(),
        ash::ext::mesh_shader::NAME.as_ptr(),
        // ash::khr::performance_query::NAME.as_ptr(), // what the fuck, why doesn't it work. Mesa
        // was supposed to support performance queries
    ];

    let mut vk12_physical_device_features = vk::PhysicalDeviceVulkan12Features::default()
        .buffer_device_address(true)
        // bindless
        .runtime_descriptor_array(true)
        .descriptor_binding_partially_bound(true)
        .shader_sampled_image_array_non_uniform_indexing(true)
        .descriptor_binding_sampled_image_update_after_bind(true);

    let mut vk13_physical_device_features = vk::PhysicalDeviceVulkan13Features::default()
        .dynamic_rendering(true)
        .maintenance4(true);

    let mut mesh_shading_features = vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
        .mesh_shader(true)
        .task_shader(true);

    let logical_device_create_info = vk::DeviceCreateInfo::default()
        .push_next(&mut vk12_physical_device_features)
        .push_next(&mut vk13_physical_device_features)
        .push_next(&mut mesh_shading_features)
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions);

    unsafe { instance.create_device(physical_device, &logical_device_create_info, None) }
        .expect("Failed to create logical device")
}
