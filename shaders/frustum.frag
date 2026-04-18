#version 460

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set_traditional.glsl"

layout(location = 0) in vec4 in_color;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = in_color;
}
