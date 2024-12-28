#version 450

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) in vec3 in_uvw;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(texture(skybox_tx[push_constants.skybox_data.current_texture_id], in_uvw).rgb, 1.0);
}
