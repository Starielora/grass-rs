use ash::vk;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub pos: glm::Vec3,
    pub norm: glm::Vec3,
    pub tx: glm::Vec2,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Meshlet {
    pub vertex_offset: u32, // offset into meshlet_vertices array (not actual vertex buffer)
    pub triangle_offset: u32, // offset into meshlet_triangles array
    pub vertex_count: u32,
    pub triangle_count: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct MeshInstance {
    pub transform: glm::Mat4,
    pub meshlets_offset: u32, // offset in global buffer
    pub meshlets_count: u32,
}

const _: () = assert!(std::mem::size_of::<MeshInstance>() == 80);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MeshletInstance {
    pub mesh_instance_index: u32, // index into the GeometryInstance buffer (keeps transform + Geometry ref)
    pub meshlet_index: u32,       // global index into the meshlets buffer
}

pub struct GlobalGeometryData {
    pub vertices: std::vec::Vec<Vertex>,
    pub meshlet_vertices: std::vec::Vec<u32>,
    pub meshlet_triangles: std::vec::Vec<u8>,
    pub meshlets: std::vec::Vec<Meshlet>,
    pub mesh_instances: std::vec::Vec<MeshInstance>,
    pub meshlet_instances: std::vec::Vec<MeshletInstance>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PushConstants {
    pub view_camera: vk::DeviceAddress,
    pub vertices: vk::DeviceAddress,
    pub meshlet_vertices: vk::DeviceAddress,
    pub meshlet_triangles: vk::DeviceAddress,
    pub meshlets: vk::DeviceAddress,
    pub mesh_instances: vk::DeviceAddress,
    pub meshlet_instances: vk::DeviceAddress,
    pub mesh_instances_count: u32,
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
