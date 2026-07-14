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

    // culling data
    pub bounding_sphere_center: glm::Vec3,
    pub bounding_sphere_radius: f32,
    pub cone_apex: glm::Vec3,
    pub cone_cutoff: f32,
    pub cone_axis: glm::Vec3,
    pub _padding: f32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MeshLod {
    pub meshlets_offset: u32, // offset in global buffer
    pub meshlets_count: u32,
}

pub const MAX_LODS: usize = 8;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Mesh {
    pub mesh_lods: [MeshLod; MAX_LODS],
    pub lod_count: u32,
    pub _padding: glm::Vec3,

    pub bounding_sphere_center: glm::Vec3,
    pub bounding_sphere_radius: f32,
}

const _: () = assert!(std::mem::size_of::<Mesh>() == 96);

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct MeshInstance {
    pub transform: glm::Mat4,
    pub mesh_index: u32,
    pub _padding: u32,
}

const _: () = assert!(std::mem::size_of::<MeshInstance>() == 80);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MeshletInstance {
    pub mesh_instance_index: u32, // index into the GeometryInstance buffer (keeps transform + Geometry ref)
    pub meshlet_index: u32,       // global index into the meshlets buffer
    pub lod_index: u32,
}

pub struct GeometryBuildData {
    pub vertices: std::vec::Vec<Vertex>,
    pub meshlet_vertices: std::vec::Vec<u32>,
    pub meshlet_triangles: std::vec::Vec<u8>,
    pub meshes: std::vec::Vec<Mesh>,
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
    pub meshes: vk::DeviceAddress,
    pub meshlets: vk::DeviceAddress,
    pub mesh_instances: vk::DeviceAddress,
    pub meshlet_instances: vk::DeviceAddress,
    pub draw_mesh_tasks_commands: vk::DeviceAddress,
    pub visible_meshlet_instances: vk::DeviceAddress,
    pub visible_meshlet_instances_count: vk::DeviceAddress,
    pub mesh_instances_count: u32,
    pub meshlet_instances_count: u32,
    pub draws_count: u32,
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

pub(super) fn create_pipeline_layout(
    vk: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let set_layouts = [descriptor_set_layout];
    let push_constants_range = get_push_constant_range();
    let create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constants_range);
    unsafe {
        vk.create_pipeline_layout(&create_info, None)
            .expect("Failed to create pipeline layout")
    }
}

pub fn task_dispatch_2d(instance_count: u32, subgroup_size: u32, max_dim: u32) -> (u32, u32) {
    let total_groups = (instance_count + (subgroup_size - 1)) / subgroup_size;
    let group_x = total_groups.min(max_dim);
    let group_y = (total_groups + max_dim - 1) / max_dim;

    (group_x, group_y)
}
