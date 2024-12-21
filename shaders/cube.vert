#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) out vec3 frag_pos;
layout(location = 1) out vec3 frag_normal;

void main()
{
    VertexData d = push_constants.cube_vertex_data.data[gl_VertexIndex];
    vec4 vertex = vec4(d.pos_x, d.pos_y, d.pos_z, 1.0);

    frag_pos = vec3(push_constants.cube_model.matrix * vertex);

    vec3 normal = vec3(d.norm_x, d.norm_y, d.norm_z);

    frag_normal = mat3(transpose(inverse(push_constants.cube_model.matrix))) * normal;

    gl_Position = push_constants.camera_data.projview * vec4(frag_pos, 1.0);
}
