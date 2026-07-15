use ash::vk;

use crate::{
    meshlet2::{
        compute::pipeline,
        gpu::{self, CPUPushConstant},
    },
    vkutils::shaders,
};

pub struct Pipeline {
    vk: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    push_constant: PushConstant,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PushConstant {
    pub _visible_meshlet_instances_count: vk::DeviceAddress,
    pub _draw_mesh_tasks_commands: vk::DeviceAddress,
}

impl gpu::CPUPushConstant for PushConstant {
    fn stage_flags() -> vk::ShaderStageFlags {
        vk::ShaderStageFlags::COMPUTE
    }
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
            gpu::create_pipeline_layout(vk, descriptor_set_layout, PushConstant::range());
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
