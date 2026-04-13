#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set_common.glsl"

struct Vertex {
    float vx, vy, vz;
    float nx, ny, nz;
    float tx, ty;
};

layout(buffer_reference) readonly buffer VertexBuf {
    Vertex vertices[];
};

layout(buffer_reference) readonly buffer TriangleIndexBuf {
    uint8_t meshlet_triangles[];
};

layout(buffer_reference) readonly buffer VertexIndexBuf {
    uint meshlet_vertices[];
};

struct MeshletBounds {
    vec3 center;
    float radius;
    vec3 cone_apex;
    float cone_cutoff;
    vec3 cone_axis;
    // TODO its actually signed int - I wanted to avoid another glsl extension
    float remaining_cone_data;
    // uint8_t cone_axis_s8;
    // uint8_t cone_cutoff_s8;
};

layout(buffer_reference) readonly buffer MeshletBoundsBuf {
    MeshletBounds bounds[];
};

struct Meshlet {
    uint vertex_offset;
    uint triangle_offset;
    uint vertex_count;
    uint triangle_count;
};

layout(buffer_reference) readonly buffer MeshletBuf {
    Meshlet meshlets[];
};

// One entry per indirect draw call in the meshlet path.
struct MeshletDraw {
    TransformBuf transform; // per-instance model matrix
    MeshletBuf meshlets;
    VertexBuf vertices;
    VertexIndexBuf vertex_indices;
    TriangleIndexBuf tri_indices;
    MeshletBoundsBuf bounds;
    uint meshlets_count;
};

layout(buffer_reference) readonly buffer MeshletDrawBuf {
    MeshletDraw draws[];
};

layout(push_constant) uniform constants
{
    CameraDataBuf camera;      // view camera: vertex transform
    CameraDataBuf cull_camera; // cull camera: cone/frustum culling
    MeshletDrawBuf meshlet_draws;
} push_constants;
