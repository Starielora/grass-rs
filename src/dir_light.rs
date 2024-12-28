extern crate nalgebra_glm as glm;

use ash::vk;

use crate::{gui_scene_node::GuiSceneNode, vkutils};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GPUDirLight {
    pub dir: glm::Vec4,
    pub color: glm::Vec4,
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

        let (buffer, memory, allocation_size, buffer_ptr) = vkctx.create_bar_buffer(
            std::mem::size_of::<GPUDirLight>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

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

    fn update_gpu_buffer(self: &Self) {
        unsafe {
            ash::util::Align::new(
                self.buffer_ptr,
                std::mem::align_of::<GPUDirLight>() as u64,
                self.buffer_allocation_size,
            )
            .copy_from_slice(&[self.gpu_data]);
        }
    }
}

impl GuiSceneNode for DirLight {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        let mut changed = [false, false];

        if ui.tree_node("Directional light").is_some() {
            ui.indent();
            changed[0] = imgui::Drag::new("Direction")
                .range(-1.0, 1.0)
                .speed(0.1)
                .build_array(ui, &mut self.gpu_data.dir.data.0[0]);

            changed[1] = ui.color_edit4("Color", &mut self.gpu_data.color.data.0[0]);
            ui.unindent();
        }

        if changed.contains(&true) {
            self.update_gpu_buffer();
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
