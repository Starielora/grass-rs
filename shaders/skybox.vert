#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 tx;

layout(location = 0) out vec3 out_uvw;

void main()
{
    vec4 vertex = vec4(pos, 1.0);

    out_uvw = vertex.rgb;

    vertex.xyz *= 200;

    gl_Position = push_constants.camera_data.projview * (vertex + push_constants.camera_data.position);
}
