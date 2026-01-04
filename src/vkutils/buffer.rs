use super::vk_destroy;
use ash::vk;

pub struct Buffer {
    pub handle: vk::Buffer,
    memory: vk::DeviceMemory,
    pub _allocation_size: u64,
    pub device_address: Option<vk::DeviceAddress>,
    pub ptr: Option<*mut std::ffi::c_void>,
    device: ash::Device,
}

impl Buffer {
    pub fn new(
        device: ash::Device,
        size: usize,
        usage: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
        physical_device_props: &vk::PhysicalDeviceProperties,
        memory_props: &vk::PhysicalDeviceMemoryProperties,
    ) -> Self {
        let aligned_size = pad_buffer_size(size as u64, physical_device_props);

        let create_info = vk::BufferCreateInfo {
            flags: vk::BufferCreateFlags::empty(),
            size: aligned_size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.create_buffer(&create_info, None).unwrap() };
        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let with_device_address = usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS);

        let memory = super::device_memory::allocate(
            &device,
            &memory_requirements,
            &memory_props,
            memory_property_flags,
            with_device_address,
        );

        unsafe { device.bind_buffer_memory(buffer, memory, 0).unwrap() };

        let mut device_address: Option<vk::DeviceAddress> = None;
        if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
            device_address = Some(get_buffer_device_address(&device, buffer));
        }

        let mut ptr: Option<*mut std::ffi::c_void> = None;
        if memory_property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            ptr = Some(map_buffer(&device, memory));
        }

        Self {
            handle: buffer,
            memory,
            _allocation_size: memory_requirements.size,
            device_address,
            ptr,
            device,
        }
    }

    pub fn update_contents<T: std::marker::Copy>(&self, slice: &[T]) {
        let ptr = self.ptr.unwrap_or_else(|| {
            panic!("Not a mapped buffer.");
        });

        // TODO alignment + size boundary check?

        unsafe {
            let mapped_slice = core::slice::from_raw_parts_mut(ptr.cast(), slice.len());
            mapped_slice.copy_from_slice(slice);
        }
    }

    pub fn unmap_memory(&mut self) {
        unsafe {
            self.device.unmap_memory(self.memory);
            self.ptr = None;
        }
    }
}

impl vk_destroy::VkDestroy for Buffer {
    fn vk_destroy(&self) {
        unsafe {
            self.device.free_memory(self.memory, None);
            self.device.destroy_buffer(self.handle, None);
        }
    }
}

fn pad_buffer_size(size: u64, physical_device_props: &vk::PhysicalDeviceProperties) -> u64 {
    let min_buffer_alignment = physical_device_props
        .limits
        .min_uniform_buffer_offset_alignment;

    let mut aligned_size = size;

    if min_buffer_alignment > 0 {
        aligned_size = (aligned_size + min_buffer_alignment - 1) & !(min_buffer_alignment - 1);
    }

    aligned_size
}

fn get_buffer_device_address(device: &ash::Device, buffer: vk::Buffer) -> vk::DeviceAddress {
    let buffer_address_info = vk::BufferDeviceAddressInfo {
        buffer,
        ..Default::default()
    };

    unsafe { device.get_buffer_device_address(&buffer_address_info) }
}

fn map_buffer(device: &ash::Device, memory: vk::DeviceMemory) -> *mut std::ffi::c_void {
    unsafe {
        device
            .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
            .expect("Failed to map memory.")
    }
}
