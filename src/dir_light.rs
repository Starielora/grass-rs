extern crate nalgebra_glm as glm;

use ash::vk;

use crate::{
    gui_scene_node::GuiSceneNode,
    vkutils,
    vkutils_new::{self, vk_destroy::VkDestroy},
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GPUDirLight {
    pub dir: glm::Vec4,
    pub color: glm::Vec4,
}

pub struct DirLight {
    pub gpu_data: GPUDirLight,
    buffer: vkutils_new::buffer::Buffer,
    pub buffer_device_address: vk::DeviceAddress,
}

impl DirLight {
    pub fn new(data: GPUDirLight, vkctx: &vkutils::Context) -> DirLight {
        let buffer = vkctx.create_bar_buffer(
            std::mem::size_of::<GPUDirLight>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let buffer_device_address = buffer.device_address.unwrap();

        buffer.update_contents(&[data]);

        Self {
            gpu_data: data,
            buffer,
            buffer_device_address,
        }
    }

    fn update_gpu_buffer(self: &Self) {
        self.buffer.update_contents(&[self.gpu_data]);
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
        self.buffer.vk_destroy();
    }
}
