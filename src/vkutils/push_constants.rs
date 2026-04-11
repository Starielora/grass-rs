use ash::vk;

extern crate nalgebra_glm as glm;

#[derive(Clone, Default)]
#[repr(C)]
pub struct GPUPushConstants {
    pub mesh_transform: vk::DeviceAddress, // TransformBuf  (currently unused at runtime)
    pub camera: vk::DeviceAddress,         // CameraDataBuf
    pub dir_light_camera: vk::DeviceAddress, // CameraDataBuf
    pub dir_light: vk::DeviceAddress,      // DirLightBuf
    pub skybox: vk::DeviceAddress,         // SkyboxBuf
    // Meshlet path only:
    pub meshlet_draws: vk::DeviceAddress, // MeshletDrawBuf
    // Traditional path only:
    pub instances: vk::DeviceAddress, // TraditionalInstanceBuf
    pub instance_offsets: vk::DeviceAddress, // TraditionalOffsetBuf
    // Shared:
    pub depth_sampler_index: u32,
}

// TODO why I cannot define this as static or const array is beyond me. It says I cannot use
// non-const shit in static context, and yet ShaderStageFlags are const.
pub fn get_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX
            | vk::ShaderStageFlags::FRAGMENT
            | vk::ShaderStageFlags::TASK_EXT
            | vk::ShaderStageFlags::MESH_EXT,
        offset: 0,
        size: std::mem::size_of::<GPUPushConstants>() as u32,
    }]
}
