#version 450

layout(set = 0, binding = 0) uniform samplerCube skybox_tx;

layout(location = 0) in vec3 in_uvw;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(texture(skybox_tx, in_uvw).rgb, 1.0);
}
