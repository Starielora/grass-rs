extern crate nalgebra_glm as glm;

use ash::vk;

use crate::{
    camera::{self, GPUCameraData},
    gui_scene_node::GuiSceneNode,
    vkutils::{self, vk_destroy::VkDestroy},
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GPUDirLight {
    pub dir: glm::Vec4,
    pub color: glm::Vec4,
}

pub struct GuiData {
    pub azimuth_deg: [f32; 1],
    pub inclination_deg: [f32; 1],
    pub distance: [f32; 1],
    pub xz_target: [f32; 2],
}

pub struct DirLight {
    pub gpu_data: GPUDirLight,
    gui_data: GuiData,
    buffer: vkutils::buffer::Buffer,
    pub buffer_device_address: vk::DeviceAddress,
    pub camera_buffer: vkutils::buffer::Buffer,
    pub depth_image: vkutils::image::Image,
    w: f32,
    h: f32,
}

impl DirLight {
    pub fn new(mut data: GPUDirLight, ctx: &vkutils::context::VulkanContext) -> Self {
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

        let gui_data = GuiData {
            azimuth_deg: [30.0],
            inclination_deg: [45.0],
            distance: [1.6],
            xz_target: [0.0, 0.0],
        };

        update_gpu_buffers(
            &buffer,
            &camera_buffer,
            &gui_data,
            &mut data,
            (
                ctx.swapchain.extent.width as f32,
                ctx.swapchain.extent.height as f32,
            ),
        );

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
            buffer,
            gpu_data: data,
            gui_data,
            buffer_device_address,
            camera_buffer,
            depth_image,
            w: ctx.swapchain.extent.width as f32,
            h: ctx.swapchain.extent.height as f32,
        }
    }
}

fn update_gpu_buffers(
    dir_light_buffer: &vkutils::buffer::Buffer,
    camera_buffer: &vkutils::buffer::Buffer,
    gui_data: &GuiData,
    gpu_data: &mut GPUDirLight,
    (w, h): (f32, f32),
) {
    let azimuth = (gui_data.azimuth_deg[0]).to_radians();
    let inclination = (gui_data.inclination_deg[0]).to_radians();

    let mut mat = glm::Mat4::identity();
    mat = glm::rotate(&mat, azimuth, &glm::make_vec3(&[0.0, -1.0, 0.0]));
    mat = glm::rotate(&mat, inclination, &glm::make_vec3(&[0.0, 0.0, 1.0]));

    gpu_data.dir = mat * glm::make_vec4(&[-1.0, 0.0, 0.0, 0.0]);

    dir_light_buffer.update_contents(&[*gpu_data]);

    let (view_matrix, pos) = camera::view::from_spherical(
        azimuth,
        inclination,
        gui_data.distance[0],
        glm::make_vec3(&[gui_data.xz_target[0], gui_data.xz_target[1], 0.0]),
    );

    let pos = glm::make_vec4(&[pos.x, pos.y, pos.z, 0.0]);

    let projection_matrix = camera::projection::Projection::Orthographic(
        camera::projection::orthtographic::Properties::new(w, h, gui_data.distance[0]),
    )
    .compute_matrix();

    let projview = projection_matrix * view_matrix;

    let camera_gpu_data = GPUCameraData { pos, projview };

    camera_buffer.update_contents(&[camera_gpu_data]);
}

impl GuiSceneNode for DirLight {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        let mut changed = [false, false, false, false, false];

        if ui
            .tree_node_config("Directional light")
            .opened(true, imgui::Condition::Appearing)
            .push()
            .is_some()
        {
            ui.indent();
            changed[0] = imgui::Drag::new("Azimuth")
                .range(-180.0, 180.0)
                .speed(1.0)
                .build_array(ui, &mut self.gui_data.azimuth_deg);

            changed[1] = imgui::Drag::new("Inclination")
                // TODO use quaternions - gimbal lock :(
                .range(-89.99, 89.99)
                .speed(1.0)
                .build_array(ui, &mut self.gui_data.inclination_deg);

            changed[2] = imgui::Drag::new("Distance")
                .range(0.0, 500.0)
                .speed(1.0)
                .build_array(ui, &mut self.gui_data.distance);

            changed[3] = imgui::Drag::new("XZ target")
                .range(-100.0, 100.0)
                .speed(1.0)
                .build_array(ui, &mut self.gui_data.xz_target);

            changed[4] = ui.color_edit4("Color", &mut self.gpu_data.color.data.0[0]);
            ui.unindent();
        }

        if changed.contains(&true) {
            update_gpu_buffers(
                &self.buffer,
                &self.camera_buffer,
                &self.gui_data,
                &mut self.gpu_data,
                (self.w, self.h),
            );
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
