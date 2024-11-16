layout(set = 0, binding = 0) uniform CameraData
{
    vec4 position;
    mat4 projview;
} camera_data;

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

layout(set = 0, binding = 1) readonly buffer ObjectBuffer {
    VertexData data[];
} object_buffer;

layout(push_constant) uniform constants
{
    mat4 cube_model;
} push_constants;
