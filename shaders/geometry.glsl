#extension GL_EXT_buffer_reference: require
#extension GL_EXT_shader_8bit_storage : require

struct Vertex {
    float vx, vy, vz;
    float nx, ny, nz;
    float tx, ty;
};

struct Meshlet {
    uint vertex_offset;
    uint triangle_offset;
    uint vertex_count;
    uint triangle_count;

    vec3 bounding_sphere_center;
    float bounding_sphere_radius;
    vec3 cone_apex;
    float cone_cutoff;
    vec3 cone_axis;
    float _padding;
};

struct MeshLod {
    uint meshlets_offset;
    uint meshlets_count;
};

struct Mesh {
    MeshLod lod[8];
    uint lod_count;
    uint _pad0;
    uint _pad1;
    uint _pad2;

    vec3 bounding_sphere_center;
    float bounding_sphere_radius;
};

struct MeshInstance {
    mat4 transform;
    uint mesh_index;
    uint _padding;
};

struct MeshletInstance {
    uint mesh_instance_index;
    uint meshlet_index;
    uint lod_index; // TODO remove this once LOD is implemented on compute prepass
};

// This is basically VkDrawMeshTasksIndirectCommandEXT
struct DrawMeshTasksCommand {
    uint groupCountX;
    uint groupCountY;
    uint groupCountZ;
};

layout(buffer_reference) readonly buffer VertexBuf {
    Vertex items[];
};

layout(buffer_reference) readonly buffer MeshletsTrianglesBuf {
    uint8_t items[];
};

layout(buffer_reference) readonly buffer MeshletsVerticesBuf {
    uint items[];
};

layout(buffer_reference) readonly buffer MeshletsBuf {
    Meshlet items[];
};

layout(buffer_reference) readonly buffer MeshInstanceBuf {
    MeshInstance items[];
};

layout(buffer_reference) readonly buffer MeshletInstanceBuf {
    MeshletInstance items[];
};

layout(buffer_reference) readonly buffer MeshesBuf {
    Mesh items[];
};

layout(buffer_reference) buffer DrawMeshTasksCommandBuf {
    DrawMeshTasksCommand items[];
};

layout(buffer_reference) buffer VisibleMeshletInstancesBuf {
    MeshletInstance items[];
};

layout(buffer_reference) buffer VisibleMeshletInstancesCountBuf {
    uint count;
};

