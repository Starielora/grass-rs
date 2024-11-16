#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

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
