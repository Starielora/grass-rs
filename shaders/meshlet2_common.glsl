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
};

struct MeshInstance {
    mat4 transform;
    uint meshlets_offset;
    uint meshlets_count;
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

layout(push_constant) uniform constants
{
    CameraDataBuf view_camera;
    VertexBuf vertices;
    MeshletsVerticesBuf meshlets_vertices;
    MeshletsTrianglesBuf meshlets_triangles;
    MeshletsBuf meshlets;
    MeshInstanceBuf mesh_instances;
    MeshletInstanceBuf meshlet_instances;
    uint mesh_instances_count;
    uint meshlet_instances_count;
} push_constants;

struct TaskPayload {
    uint meshlet_instance_index[64]; // must be >= task shader local_size_x
};
