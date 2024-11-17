#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

vec2 positions[3] = vec2[](
    vec2(0.0, 0.5),
    vec2(0.5, -0.5),
    vec2(-0.5, -0.5)
);

vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

layout(location = 0) out vec3 fragColor;

void main()
{
    CameraData camera_data = push_constants.camera_data;
    gl_Position = camera_data.projview * vec4(positions[gl_VertexIndex], 0.0, 1.0);
    fragColor = colors[gl_VertexIndex];
}
