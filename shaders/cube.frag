#version 450

layout(location = 0) out vec4 out_color;

layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec4 frag_normal;

void main() {
    out_color = vec4(frag_normal.xyz * 0.5 + 0.5, 1.0);
}

