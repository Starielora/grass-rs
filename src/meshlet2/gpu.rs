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
}

const _: () = assert!(std::mem::size_of::<MeshInstance>() == 64);

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
