use ash::vk;

use crate::{meshlet2::compute::pipeline, vkutils::shaders};

pub struct Pipeline {
    vk: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    push_constant: PushConstant,
}

pub struct PushConstant {
    pub _visible_meshlet_instances_count: vk::DeviceAddress,
    pub _draw_mesh_tasks_commands: vk::DeviceAddress,
}

impl std::ops::Drop for Pipeline {
    fn drop(&mut self) {
        let vk = &self.vk;
        unsafe {
            vk.destroy_pipeline(self.pipeline, None);
            vk.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

impl Pipeline {
    pub fn new(
        vk: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
        subgroup_size: u32,
        visible_meshlet_instances_count_bda: vk::DeviceAddress,
        draw_mesh_tasks_commands_bda: vk::DeviceAddress,
    ) -> Self {
        let pipeline_layout =
            pipeline::create_pipeline_layout(vk, descriptor_set_layout, PushConstant::get_range());
        let pipeline = pipeline::create_pipeline(
            vk,
            pipeline_layout,
            &shaders::PREP_DRAW_MESH_TASKS_COMMAND,
            subgroup_size,
        );

        Self {
            vk: vk.clone(),
            pipeline,
            pipeline_layout,
            push_constant: PushConstant {
                _visible_meshlet_instances_count: visible_meshlet_instances_count_bda,
                _draw_mesh_tasks_commands: draw_mesh_tasks_commands_bda,
            },
        }
    }

    pub fn record(&self, command_buffer: vk::CommandBuffer) {
        let vk = &self.vk;
        unsafe {
            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline,
            );

            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                self.push_constant.data(),
            );

            vk.cmd_dispatch(command_buffer, 1, 1, 1);
        }
    }
}

impl PushConstant {
    pub fn data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const PushConstant) as *const u8,
                std::mem::size_of::<PushConstant>(),
            )
        }
    }

    pub fn get_range() -> vk::PushConstantRange {
        vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<Self>() as u32,
        }
    }
}
