#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require
#extension GL_EXT_shader_8bit_storage : require

layout(set = 0, binding = 0) uniform samplerCube skybox_tx[];
layout(set = 0, binding = 1) uniform sampler2D depth_textures[];

layout(buffer_reference) readonly buffer CameraDataBuf {
    vec4 position;
    mat4 projview;
};

// Per-instance model transform. Both paths store an address to
// one of these per scene node.
layout(buffer_reference) readonly buffer TransformBuf {
    mat4 model_matrix;
};

struct DirLight {
    vec4 dir;
    vec4 color;
};

layout(buffer_reference) readonly buffer DirLightBuf {
    DirLight data;
};

layout(buffer_reference) readonly buffer SkyboxBuf {
    uint current_texture_id;
};

// -------------------------------------------------------------
// Meshlet rendering path — used only by meshlet.task / meshlet.mesh
// -------------------------------------------------------------

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

// -------------------------------------------------------------
// Traditional rendering path — used only by cube.vert / cube.frag
// -------------------------------------------------------------

// Array of per-instance transforms, indexed by instance index.
layout(buffer_reference) readonly buffer TraditionalInstanceBuf {
    TransformBuf transforms[];
};

// Per-draw-call offset into the instance buffer, indexed by gl_DrawID.
layout(buffer_reference) readonly buffer TraditionalOffsetBuf {
    uint offset[];
};

layout(push_constant) uniform constants
{
    TransformBuf mesh_transform; // currently unused at runtime
    CameraDataBuf camera;
    CameraDataBuf dir_light_camera;
    DirLightBuf dir_light;
    SkyboxBuf skybox;
    // Meshlet path only:
    MeshletDrawBuf meshlet_draws;
    // Traditional path only:
    TraditionalInstanceBuf instances;
    TraditionalOffsetBuf instance_offsets;
    // Shared:
    uint depth_sampler_index;
} push_constants;
