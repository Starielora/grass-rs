#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) out vec3 out_uvw;

void main()
{
    VertexData d = push_constants.cube_vertex_data.data[gl_VertexIndex];
    vec4 vertex = vec4(d.pos_x, d.pos_y, d.pos_z, 1.0);

    out_uvw = vertex.rgb;

    vertex.xyz *= 200;

    gl_Position = push_constants.camera_data.projview * (vertex + push_constants.camera_data.position);
}
