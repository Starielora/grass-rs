#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) out vec4 frag_pos;
layout(location = 1) out vec4 frag_normal;

void main()
{
    VertexData d = push_constants.cube_vertex_data.data[gl_VertexIndex];
    vec4 vertex = vec4(d.pos_x, d.pos_y, d.pos_z, 1.0);

    frag_pos = push_constants.cube_model.matrix * vertex;

    vec4 normal = vec4(d.norm_x, d.norm_y, d.norm_z, 0.0);

    frag_normal = transpose(inverse(push_constants.cube_model.matrix)) * normal;

    gl_Position = push_constants.camera_data.projview * frag_pos;
}
