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

struct Geometry {
    uint vertex_offset;
    uint meshlets_offset;
    uint meshlets_count;
};

struct GeometryInstance {
    mat4 transform;
    uint index; // index into geometry array
};

struct MeshletInstance {
    uint geometry_instance_index;
    uint meshlet_index;
};

layout(buffer_reference) readonly buffer VertexBuf {
    Vertex vertices[];
};

layout(buffer_reference) readonly buffer MeshletsTrianglesBuf {
    uint8_t meshlets_triangles[];
};

layout(buffer_reference) readonly buffer MeshletsVerticesBuf {
    uint meshlet_vertices[];
};

layout(buffer_reference) readonly buffer MeshletsBuf {
    Meshlet meshlets[];
};

layout(buffer_reference) readonly buffer GeometryBuf {
    Geometry geometry[];
};

layout(buffer_reference) readonly buffer GeometryInstanceBuf {
    GeometryInstance geometry_instance[];
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
    GeometryBuf geometry;
    GeometryInstanceBuf geometry_instances;
    MeshletInstanceBuf meshlet_instances;
    uint meshlet_instances_count;
} push_constants;

struct TaskPayload {
    uint meshlet_instance_index[32]; // must be >= task shader local_size_x
};
