pub mod mesh_data;
pub mod pipeline;

use crate::drawable;
use crate::gui_scene_node::GuiSceneNode;
use crate::vkutils;
use crate::vkutils_new;
use crate::vkutils_new::push_constants::GPUPushConstants;
use crate::vkutils_new::vk_destroy::VkDestroy;

use ash::vk::{self, IndexType};

struct GuiData {
    pub id: usize,
    pub name: std::string::String,
    pub translation: glm::Vec3,
    pub rotation: glm::Vec3,
    pub scale: glm::Vec3,
}

pub struct Mesh {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    indices_count: usize,
    per_frame_buffer: vkutils_new::buffer::Buffer,
    per_frame_buffer_device_address: vk::DeviceAddress,
    gui_data: GuiData,
}

impl std::ops::Drop for Mesh {
    fn drop(&mut self) {
        self.per_frame_buffer.vk_destroy();
    }
}

impl Mesh {
    pub fn new2(
        mesh_data: &mesh_data::MeshData,
        ctx: &vkutils_new::context::VulkanContext,
        gui_name: &str,
    ) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let current_id: usize = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let per_frame_buffer = ctx.create_bar_buffer(
            std::mem::size_of::<glm::Mat4>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let per_frame_buffer_device_address = per_frame_buffer.device_address.unwrap();

        Self {
            device: ctx.device.clone(),
            pipeline_layout: ctx.bindless_descriptor_set.pipeline_layout,
            per_frame_buffer,
            per_frame_buffer_device_address,
            gui_data: GuiData {
                id: current_id,
                name: std::string::String::from(gui_name),
                translation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                rotation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                scale: glm::make_vec3(&[1.0, 1.0, 1.0]),
            },
            vertex_buffer: mesh_data.vertex_buffer.handle,
            index_buffer: mesh_data.index_buffer.handle,
            indices_count: mesh_data.indices_count,
        }
    }

    pub fn new(mesh_data: &mesh_data::MeshData, ctx: &vkutils::Context, gui_name: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let current_id: usize = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let per_frame_buffer = ctx.create_bar_buffer(
            std::mem::size_of::<glm::Mat4>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let per_frame_buffer_device_address = per_frame_buffer.device_address.unwrap();

        Self {
            device: ctx.device.clone(),
            pipeline_layout: ctx.bindless_descriptor_set.pipeline_layout,
            per_frame_buffer,
            per_frame_buffer_device_address,
            gui_data: GuiData {
                id: current_id,
                name: std::string::String::from(gui_name),
                translation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                rotation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                scale: glm::make_vec3(&[1.0, 1.0, 1.0]),
            },
            vertex_buffer: mesh_data.vertex_buffer.handle,
            index_buffer: mesh_data.index_buffer.handle,
            indices_count: mesh_data.indices_count,
        }
    }

    pub fn set_transformation(
        self: &mut Self,
        translation: glm::Vec3,
        rotation: glm::Vec3,
        scale: glm::Vec3,
    ) {
        self.gui_data.translation = translation;
        self.gui_data.rotation = rotation;
        self.gui_data.scale = scale;

        self.refresh_per_frame_buffer();
    }

    fn refresh_per_frame_buffer(&self) {
        let model = glm::Mat4::identity();

        let model_translated = glm::translate(&model, &self.gui_data.translation);

        let mut model_rotated = glm::rotate(
            &model_translated,
            self.gui_data.rotation.x,
            &glm::make_vec3(&[1.0, 0.0, 0.0]),
        );

        model_rotated = glm::rotate(
            &model_rotated,
            self.gui_data.rotation.y,
            &glm::make_vec3(&[0.0, 1.0, 0.0]),
        );

        model_rotated = glm::rotate(
            &model_rotated,
            self.gui_data.rotation.z,
            &glm::make_vec3(&[0.0, 0.0, 1.0]),
        );

        let model_scaled = glm::scale(&model_rotated, &self.gui_data.scale);

        self.per_frame_buffer.update_contents(&[model_scaled]);
    }

    pub fn cmd_draw2(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        push_constants: &mut GPUPushConstants,
    ) {
        unsafe {
            push_constants.mesh_data = self.per_frame_buffer_device_address;

            device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            let vertex_buffers = [self.vertex_buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer, 0, IndexType::UINT16);
            device.cmd_draw_indexed(command_buffer, self.indices_count as u32, 1, 0, 0, 0);
        }
    }
}

impl drawable::Drawable for Mesh {
    fn cmd_draw(
        &mut self,
        command_buffer: vk::CommandBuffer,
        _pipeline: vk::Pipeline,
        push_constants: &mut GPUPushConstants,
    ) {
        unsafe {
            push_constants.mesh_data = self.per_frame_buffer_device_address;

            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (push_constants as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            let vertex_buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer,
                0,
                IndexType::UINT16,
            );
            self.device
                .cmd_draw_indexed(command_buffer, self.indices_count as u32, 1, 0, 0, 0);
        }
    }
}

impl GuiSceneNode for Mesh {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        let mut changed = [false, false, false];

        if ui.tree_node(format!("{}", self.gui_data.name)).is_some() {
            ui.indent();
            changed[0] = imgui::Drag::new(format!("Translation##{}", self.gui_data.id))
                .range(-50.0, 50.0)
                .speed(0.25)
                .build_array(ui, &mut self.gui_data.translation.data.0[0]);
            changed[1] = imgui::Drag::new(format!("Rotation##{}", self.gui_data.id))
                .range(-50.0, 50.0)
                .speed(0.25)
                .build_array(ui, &mut self.gui_data.rotation.data.0[0]);
            changed[2] = imgui::Drag::new(format!("Scale##{}", self.gui_data.id))
                .range(0.0, 50.0)
                .speed(0.25)
                .build_array(ui, &mut self.gui_data.scale.data.0[0]);
            ui.unindent();
        }

        if changed.contains(&true) {
            self.refresh_per_frame_buffer();
        }
    }
}
