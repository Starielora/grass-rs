#version 460 core
#extension GL_GOOGLE_include_directive : enable

#include "meshlet2_common.glsl"

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec4 color;

void main() {
    out_color = color;
}
