#include "camera_data.glsl"
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require
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

struct Mesh {
    uint meshlets_offset;
    uint meshlets_count;
    vec2 _padding;

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

layout(push_constant) uniform constants
{
    CameraDataBuf view_camera;
    VertexBuf vertices;
    MeshletsVerticesBuf meshlets_vertices;
    MeshletsTrianglesBuf meshlets_triangles;
    MeshesBuf meshes;
    MeshletsBuf meshlets;
    MeshInstanceBuf mesh_instances;
    MeshletInstanceBuf meshlet_instances;
    uint mesh_instances_count;
    uint meshlet_instances_count;
} push_constants;

struct TaskPayload {
    // This is insane, glsl. I wanted a specialized constant for this size and reuse it for local_size_x,
    // but apparently glsl sets local_size_x to a default 1, when used for local_size_x_id - does not respect the default value of actual constant, but it uses 64 for array size, wtf.
    // Instead, I'm going for a worst case scenario - 64 or 128 elements, which will fit all cases, and use gl_WorkGroupSize.x to query chosen wokrgroup size
    // ... because constant would not solve my issues, as apparently AMD GPU driver can dynamically choose a subgroup size of EITHER 32 or 64
    // And thinking about managing the requiredSubgroupSize VK extension with all that just made me give up and go as simple as possible.
    uint meshlet_instance_index[64]; // must be >= task shader local_size_x
};

layout(constant_id = 0) const uint SUBGROUP_SIZE = 1;
