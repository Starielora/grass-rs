use ash::vk;

pub fn allocate(
    device: &ash::Device,
    memory_requirements: &vk::MemoryRequirements,
    memory_props: &vk::PhysicalDeviceMemoryProperties,
    memory_property_flags: vk::MemoryPropertyFlags,
    with_device_address: bool,
) -> vk::DeviceMemory {
    let memory_type_index = find_memory_type(
        memory_props,
        memory_requirements.memory_type_bits,
        memory_property_flags,
    );

    let mut allocate_info = vk::MemoryAllocateInfo {
        allocation_size: memory_requirements.size,
        memory_type_index,
        ..Default::default()
    };

    let mut device_address_allocate_flags =
        vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);

    if with_device_address {
        // TODO this situation looks like it could be done better
        allocate_info = allocate_info.push_next(&mut device_address_allocate_flags);
    }

    unsafe { device.allocate_memory(&allocate_info, None) }.expect("Failed to allocate memory")
}

pub(super) fn find_memory_type(
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

    panic!("Failed to find memory type.");
}
