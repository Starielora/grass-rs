use ash::vk;

use crate::{
    meshlet2::{self, compute::pipeline},
    vkutils::shaders,
};

pub struct Pipeline {
    vk: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

#[repr(C)]
struct PushConstant {
    view_camera: vk::DeviceAddress,
    meshes: vk::DeviceAddress,
    mesh_instances: vk::DeviceAddress,
    visible_meshlet_instances: vk::DeviceAddress,
    visible_meshlet_instances_count: vk::DeviceAddress,
    mesh_instances_count: u32,
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
    ) -> Self {
        let pipeline_layout =
            pipeline::create_pipeline_layout(vk, descriptor_set_layout, PushConstant::get_range());
        let pipeline = pipeline::create_pipeline(
            vk,
            pipeline_layout,
            &shaders::COMPUTE_VISIBLE_MESHLETS_COMP,
            subgroup_size,
        );

        Self {
            vk: vk.clone(),
            pipeline,
            pipeline_layout,
        }
    }

    fn push_constant(
        &self,
        data: &meshlet2::GeometryBuffers,
        camera_bda: vk::DeviceAddress,
    ) -> PushConstant {
        PushConstant {
            view_camera: camera_bda,
            meshes: data.meshes.device_address.unwrap(),
            mesh_instances: data.mesh_instances.device_address.unwrap(),
            visible_meshlet_instances: data.visible_meshlets_instances.device_address.unwrap(),
            visible_meshlet_instances_count: data
                .visible_meshlets_instances_count
                .device_address
                .unwrap(),
            mesh_instances_count: data.mesh_instances_count,
        }
    }

    pub fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        geometry_data: &meshlet2::GeometryBuffers,
        camera_bda: vk::DeviceAddress,
    ) {
        let vk = &self.vk;
        unsafe {
            vk.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline,
            );

            let pc = self.push_constant(&geometry_data, camera_bda);

            vk.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                pc.data(),
            );

            vk.cmd_dispatch(command_buffer, geometry_data.mesh_instances_count, 1, 1);
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
