use ash::vk;

extern crate nalgebra_glm as glm;

#[repr(C)]
pub struct GPUPushConstants {
    pub cube_vertex: vk::DeviceAddress,
    pub cube_model: vk::DeviceAddress,
    pub camera_data_buffer_address: vk::DeviceAddress,
}

// TODO why I cannot define this as static or const array is beyond me. It says I cannot use
// non-const shit in static context, and yet ShaderStageFlags are const.
pub fn get_push_constants_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: std::mem::size_of::<GPUPushConstants>() as u32,
    }]
}