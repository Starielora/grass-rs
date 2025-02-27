use ash::vk;

extern crate nalgebra_glm as glm;

#[derive(Clone, Default)]
#[repr(C)]
pub struct GPUPushConstants {
    pub mesh_data: vk::DeviceAddress,
    pub camera_data_buffer_address: vk::DeviceAddress,
    pub dir_light_camera_buffer_address: vk::DeviceAddress,
    pub dir_light_buffer_address: vk::DeviceAddress,
    pub skybox_data: vk::DeviceAddress,
    pub depth_sampler_index: u32,
}

// TODO why I cannot define this as static or const array is beyond me. It says I cannot use
// non-const shit in static context, and yet ShaderStageFlags are const.
pub fn get_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: std::mem::size_of::<GPUPushConstants>() as u32,
    }]
}
