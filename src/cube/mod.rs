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

pub struct Cube {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    indices_count: usize,
    model_buffer: vk::Buffer,
    model_buffer_memory: vk::DeviceMemory,
    model_buffer_ptr: *mut std::ffi::c_void,
    model_buffer_allocation_size: u64,
    pub model_buffer_device_address: vk::DeviceAddress,
    gui_data: GuiData,
}

impl std::ops::Drop for Cube {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.model_buffer_memory, None);
            self.device.destroy_buffer(self.model_buffer, None);
        }
    }
}

impl Cube {
    pub fn new(cube_pipeline: &pipeline::Pipeline, ctx: &vkutils::Context, gui_name: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let current_id: usize = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let (model_buffer, model_buffer_memory, model_buffer_allocation_size) = ctx.create_buffer(
            std::mem::size_of::<glm::Mat4>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let model_buffer_ptr = unsafe {
            ctx.device
                .map_memory(
                    model_buffer_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Could not map cube model buffer memory")
        };

        let model_buffer_device_address = unsafe {
            let address_info = vk::BufferDeviceAddressInfo {
                buffer: model_buffer,
                ..Default::default()
            };
            ctx.device.get_buffer_device_address(&address_info)
        };

        Self {
            device: ctx.device.clone(),
            pipeline_layout: cube_pipeline.pipeline_layout,
            pipeline: cube_pipeline.pipeline,
            model_buffer,
            model_buffer_memory,
            model_buffer_ptr,
            model_buffer_allocation_size,
            model_buffer_device_address,
            gui_data: GuiData {
                id: current_id,
                name: std::string::String::from(gui_name),
                translation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                rotation: glm::make_vec3(&[0.0, 0.0, 0.0]),
                scale: glm::make_vec3(&[1.0, 1.0, 1.0]),
            },
            vertex_buffer: cube_pipeline.vertex_buffer,
            index_buffer: cube_pipeline.index_buffer,
            indices_count: cube_pipeline.indices_count,
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
    }
}

impl drawable::Drawable for Cube {
    fn cmd_draw(&mut self, command_buffer: &vk::CommandBuffer, push_constants: &GPUPushConstants) {
        unsafe {
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

            ash::util::Align::new(
                self.model_buffer_ptr,
                std::mem::align_of::<glm::Mat4>() as u64,
                self.model_buffer_allocation_size,
            )
            .copy_from_slice(&[model_scaled]);

            self.device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            // FIXME quick hack to check if it works... these are only integers so there's no perf
            // penalty
            let mut pc = (*push_constants).clone();
            pc.cube_model = self.model_buffer_device_address;

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

impl GuiSceneNode for Cube {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        if ui.tree_node(format!("{}", self.gui_data.name)).is_some() {
            ui.indent();
            imgui::Drag::new(format!("Translation##{}", self.gui_data.id))
                .range(-50.0, 50.0)
                .speed(0.25)
                .build_array(ui, &mut self.gui_data.translation.data.0[0]);
            imgui::Drag::new(format!("Rotation##{}", self.gui_data.id))
                .range(-50.0, 50.0)
                .speed(0.25)
                .build_array(ui, &mut self.gui_data.rotation.data.0[0]);
            imgui::Drag::new(format!("Scale##{}", self.gui_data.id))
                .range(0.0, 50.0)
                .speed(0.25)
                .build_array(ui, &mut self.gui_data.scale.data.0[0]);
            ui.unindent();
        }
    }
}
