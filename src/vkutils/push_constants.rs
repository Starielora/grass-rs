use ash::vk;

#[derive(Clone, Default)]
#[repr(C)]
pub struct GPUPushConstantsTraditional {
    pub mesh_transform: vk::DeviceAddress, // TransformBuf (currently unused at runtime)
    pub camera: vk::DeviceAddress,         // CameraDataBuf
    pub dir_light_camera: vk::DeviceAddress, // CameraDataBuf
    pub dir_light: vk::DeviceAddress,      // DirLightBuf
    pub skybox: vk::DeviceAddress,         // SkyboxBuf
    pub instances: vk::DeviceAddress,      // TraditionalInstanceBuf
    pub instance_offsets: vk::DeviceAddress, // TraditionalOffsetBuf
    pub depth_sampler_index: u32,
}

#[derive(Clone, Default)]
#[repr(C)]
pub struct GPUPushConstantsMeshlet {
    pub camera: vk::DeviceAddress,        // CameraDataBuf
    pub meshlet_draws: vk::DeviceAddress, // MeshletDrawBuf
}

// TODO why I cannot define this as static or const array is beyond me. It says I cannot use
// non-const shit in static context, and yet ShaderStageFlags are const.
pub fn get_range_traditional() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: std::mem::size_of::<GPUPushConstantsTraditional>() as u32,
    }]
}

pub fn get_range_meshlet() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::TASK_EXT | vk::ShaderStageFlags::MESH_EXT,
        offset: 0,
        size: std::mem::size_of::<GPUPushConstantsMeshlet>() as u32,
    }]
}
