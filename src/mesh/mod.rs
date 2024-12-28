pub mod mesh_data;
pub mod pipeline;

use crate::drawable;
use crate::gui_scene_node::GuiSceneNode;
use crate::push_constants::GPUPushConstants;
use crate::vkutils;

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
    pipeline: vk::Pipeline,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    indices_count: usize,
    per_frame_buffer: vk::Buffer,
    per_frame_buffer_memory: vk::DeviceMemory,
    per_frame_buffer_ptr: *mut std::ffi::c_void,
    per_frame_buffer_allocation_size: u64,
    per_frame_buffer_device_address: vk::DeviceAddress,
    gui_data: GuiData,
}

impl std::ops::Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.per_frame_buffer_memory, None);
            self.device.destroy_buffer(self.per_frame_buffer, None);
        }
    }
}

impl Mesh {
    pub fn new(
        mesh_data: &mesh_data::MeshData,
        mesh_pipeline: &pipeline::Pipeline,
        ctx: &vkutils::Context,
        gui_name: &str,
    ) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let current_id: usize = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let (
            per_frame_buffer,
            per_frame_buffer_memory,
            per_frame_buffer_allocation_size,
            per_frame_buffer_ptr,
        ) = ctx.create_bar_buffer(
            std::mem::size_of::<glm::Mat4>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let per_frame_buffer_device_address = ctx.get_device_address(per_frame_buffer);

        Self {
            device: ctx.device.clone(),
            pipeline_layout: mesh_pipeline.pipeline_layout,
            pipeline: mesh_pipeline.pipeline,
            per_frame_buffer,
            per_frame_buffer_memory,
            per_frame_buffer_ptr,
            per_frame_buffer_allocation_size,
            per_frame_buffer_device_address,
            gui_data: GuiData {
                id: current_id,
                name: std::string::String::from(gui_name),
                translation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                rotation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                scale: glm::make_vec3(&[1.0, 1.0, 1.0]),
            },
            vertex_buffer: mesh_data.vertex_buffer,
            index_buffer: mesh_data.index_buffer,
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

        unsafe {
            ash::util::Align::new(
                self.per_frame_buffer_ptr,
                std::mem::align_of::<glm::Mat4>() as u64,
                self.per_frame_buffer_allocation_size,
            )
            .copy_from_slice(&[model_scaled])
        };
    }
}

impl drawable::Drawable for Mesh {
    fn cmd_draw(&mut self, command_buffer: &vk::CommandBuffer, push_constants: &GPUPushConstants) {
        unsafe {
            self.device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let mut pc = (*push_constants).clone();
            pc.mesh_data = self.per_frame_buffer_device_address;

            self.device.cmd_push_constants(
                *command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    (&pc as *const GPUPushConstants) as *const u8,
                    std::mem::size_of::<GPUPushConstants>(),
                ),
            );

            let vertex_buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(*command_buffer, 0, &vertex_buffers, &offsets);
            self.device.cmd_bind_index_buffer(
                *command_buffer,
                self.index_buffer,
                0,
                IndexType::UINT16,
            );
            self.device
                .cmd_draw_indexed(*command_buffer, self.indices_count as u32, 1, 0, 0, 0);
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
