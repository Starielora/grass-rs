#version 450

layout(push_constant) uniform constants
{
    mat4 cube_model;
} push_constants;

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

layout(location = 0) out vec4 frag_color;
layout(location = 1) out vec4 frag_normal;

void main()
{
    VertexData d = object_buffer.data[gl_VertexIndex];
    vec4 vertex = vec4(d.pos_x, d.pos_y, d.pos_z, 1.0);
    gl_Position = camera_data.projview * push_constants.cube_model * vertex;

    //frag_color = vec4(vertices[gl_VertexIndex], 1.);

    vec4 normal = vec4(d.norm_x, d.norm_y, d.norm_z, 0.0);

    frag_normal = transpose(inverse(push_constants.cube_model)) * normal;
}
