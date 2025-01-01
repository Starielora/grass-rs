use ash::vk;

pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub props: vk::PhysicalDeviceProperties,
    pub memory_props: vk::PhysicalDeviceMemoryProperties,
    pub graphics_queue_family_index: u32,
    pub compute_queue_family_index: u32,
    pub transfer_queue_family_index: u32,
}

pub fn find_suitable(instance: &ash::Instance) -> PhysicalDevice {
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate physical devices.")
    };

    let mut physical_device: Option<vk::PhysicalDevice> = None;
    let required_queues = get_suggested_queues();
    let mut queues_indices: std::vec::Vec<Option<u32>> = vec![None; required_queues.len()];

    for device in physical_devices {
        // TODO I'm not good at Rust :(
        // There must be a way to not overwrite this
        queues_indices = vec![None; required_queues.len()];

        let queue_family_props =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        for (index, required_flags) in required_queues.iter().enumerate() {
            for (queue_family_index, props) in queue_family_props.iter().enumerate() {
                if props.queue_flags.eq(required_flags) {
                    queues_indices[index] = Some(queue_family_index as u32);
                    break;
                }
            }
        }

        let any_queue_missing = queues_indices.iter().any(|&opt| opt.is_none());

        if any_queue_missing {
            continue;
        } else {
            physical_device = Some(device);
            break;
        }
    }

    println!(
        "Required queues: {:#?}\nQueue indices: {:#?}",
        required_queues, queues_indices
    );

    let any_queue_missing = queues_indices.iter().any(|&opt| opt.is_none());
    if any_queue_missing {
        panic!("Failed to find a physical device with required queues");
    }

    let queues: std::vec::Vec<u32> = queues_indices.iter().map(|index| index.unwrap()).collect();

    let physical_device = physical_device.unwrap();

    let memory_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let props = unsafe { instance.get_physical_device_properties(physical_device) };

    PhysicalDevice {
        handle: physical_device,
        props,
        memory_props,
        graphics_queue_family_index: queues[0],
        compute_queue_family_index: queues[1],
        transfer_queue_family_index: queues[2],
    }
}

// This is not the best solution, but it should be fine for now
// Looking at gpuinfo a lot of gpus share these flags for some reason.
fn get_suggested_queues() -> std::vec::Vec<vk::QueueFlags> {
    let graphics_queue_flags = vk::QueueFlags::GRAPHICS
        | vk::QueueFlags::TRANSFER
        | vk::QueueFlags::COMPUTE
        | vk::QueueFlags::SPARSE_BINDING;
    let transfer_queue_flags = vk::QueueFlags::TRANSFER | vk::QueueFlags::SPARSE_BINDING;
    let compute_queue_flags =
        vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER | vk::QueueFlags::SPARSE_BINDING;

    vec![
        graphics_queue_flags,
        transfer_queue_flags,
        compute_queue_flags,
    ]
}
