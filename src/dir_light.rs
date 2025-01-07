extern crate nalgebra_glm as glm;

use ash::vk;

use crate::{
    camera::{self, GPUCameraData},
    gui_scene_node::GuiSceneNode,
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
    pub camera_buffer: vkutils_new::buffer::Buffer,
    pub depth_image: vkutils_new::image::Image,
}

impl DirLight {
    pub fn new(data: GPUDirLight, ctx: &vkutils_new::context::VulkanContext) -> Self {
        let buffer = ctx.create_bar_buffer(
            std::mem::size_of::<GPUDirLight>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let buffer_device_address = buffer.device_address.unwrap();

        buffer.update_contents(&[data]);

        let camera_buffer = ctx.create_bar_buffer(
            std::mem::size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        update_camera_buffer(&camera_buffer, &data);

        let depth_image = ctx.create_image(
            ctx.depth_format,
            ctx.swapchain.extent,
            1,
            vk::SampleCountFlags::TYPE_8,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::DEPTH,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        Self {
            gpu_data: data,
            buffer,
            buffer_device_address,
            camera_buffer,
            depth_image,
        }
    }

    fn update_gpu_buffer(self: &Self) {
        self.buffer.update_contents(&[self.gpu_data]);
        update_camera_buffer(&self.camera_buffer, &self.gpu_data);
    }
}

fn update_camera_buffer(buffer: &vkutils_new::buffer::Buffer, gpu_camera_data: &GPUDirLight) {
    let mut camera = camera::Camera::new();

    // TODO I don't quite like this function. Maybe it shouldn't be recreated and recalculated each
    // call, and also this pos/dir situation is bad. I guess dir light should be at constant
    // distance on the direction line
    camera.pos = gpu_camera_data.dir.scale(-100.0).xyz();
    camera.dir = gpu_camera_data.dir.scale(-1.0).xyz();

    let camera_gpu_data = GPUCameraData {
        pos: glm::make_vec4(&[camera.pos.x, camera.pos.y, camera.pos.z, 1.0]),
        projview: camera::Camera::projection(1.0, 1.0) * camera.view(),
    };

    buffer.update_contents(&[camera_gpu_data]);
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
        self.camera_buffer.vk_destroy();
        self.depth_image.vk_destroy();
    }
}
