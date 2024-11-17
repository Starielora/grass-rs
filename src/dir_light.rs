extern crate nalgebra_glm as glm;

use ash::vk;

use crate::vkutils;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GPUDirLight {
    pub dir: glm::Vec4,
    pub ambient: glm::Vec4,
    pub diffuse: glm::Vec4,
    pub specular: glm::Vec4,
}

pub struct DirLight {
    pub gpu_data: GPUDirLight,
    buffer: vk::Buffer,
    buffer_memory: vk::DeviceMemory,
    buffer_allocation_size: u64,
    buffer_ptr: *mut std::ffi::c_void,
    pub buffer_device_address: vk::DeviceAddress,
    device: ash::Device,
}

impl DirLight {
    pub fn new(data: GPUDirLight, vkctx: &vkutils::Context) -> DirLight {
        let device = vkctx.device.clone();

        let (buffer, memory, allocation_size) = vkctx.create_buffer(
            std::mem::size_of::<GPUDirLight>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let buffer_ptr = unsafe {
            device
                .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                .expect("Could not map cube buffer memory")
        };

        let buffer_device_address = unsafe {
            let address_info = vk::BufferDeviceAddressInfo {
                buffer,
                ..Default::default()
            };
            device.get_buffer_device_address(&address_info)
        };

        unsafe {
            ash::util::Align::new(
                buffer_ptr,
                std::mem::align_of::<GPUDirLight>() as u64,
                allocation_size,
            )
            .copy_from_slice(&[data]);
        }

        Self {
            gpu_data: data,
            buffer,
            buffer_memory: memory,
            buffer_allocation_size: allocation_size,
            buffer_ptr,
            buffer_device_address,
            device: vkctx.device.clone(),
        }
    }
}

impl std::ops::Drop for DirLight {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.buffer_memory, None);
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
