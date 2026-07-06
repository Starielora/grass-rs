use ash::vk;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PushConstants {
    pub view_camera: vk::DeviceAddress,
    pub vertices: vk::DeviceAddress,
    pub meshlet_vertices: vk::DeviceAddress,
    pub meshlet_triangles: vk::DeviceAddress,
    pub meshlets: vk::DeviceAddress,
    pub geometry_instances_transforms: vk::DeviceAddress,
    pub meshlet_instances: vk::DeviceAddress,
    pub meshlet_instances_count: u32,
}

const _: () = assert!(std::mem::size_of::<PushConstants>() <= 128);

impl PushConstants {
    pub fn data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const PushConstants) as *const u8,
                std::mem::size_of::<PushConstants>(),
            )
        }
    }
}

pub fn get_push_constants_stage_flags() -> vk::ShaderStageFlags {
    vk::ShaderStageFlags::MESH_EXT | vk::ShaderStageFlags::TASK_EXT | vk::ShaderStageFlags::FRAGMENT
}

pub fn get_push_constant_range() -> [vk::PushConstantRange; 1] {
    [vk::PushConstantRange {
        stage_flags: get_push_constants_stage_flags(),
        offset: 0,
        size: std::mem::size_of::<PushConstants>() as u32,
    }]
}
