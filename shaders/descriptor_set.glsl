#extension GL_EXT_buffer_reference : require

layout(buffer_reference, std430) readonly buffer CameraData {
    vec4 position;
    mat4 projview;
};

struct VertexData {
    float pos_x;
    float pos_y;
    float pos_z;
    float norm_x;
    float norm_y;
    float norm_z;
    float tex_u;
    float tex_v;
};

layout(buffer_reference, std430) readonly buffer CubeVertexData {
    VertexData data[];
};

layout(buffer_reference, std430) readonly buffer CubeModel {
    mat4 matrix;
};

struct DirLight {
    vec4 dir;
    vec4 color;
};

layout(buffer_reference, std430) readonly buffer DirLightBuffer {
    DirLight data;
};

layout(push_constant) uniform constants
{
    CubeVertexData cube_vertex_data;
    CubeModel cube_model;
    CameraData camera_data;
    DirLightBuffer dir_light;
} push_constants;
