#version 450

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

float LinearizeDepth(float depth)
{
// TODO PARAMETERIZE THIS
  float n = 0.01;
  float f = 500.0;
  float z = depth;
  return (2.0 * n) / (f + n - z * (f - n));	
}

void main() {
    
    // TODO parameterize resource_id
	float depth = texture(depth_textures[push_constants.depth_sampler_index], in_uv).r;
    out_color = vec4(vec3(1.0 - LinearizeDepth(depth)), 1.0);
}
