#version 450

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
layout(set = 0, binding = 0) uniform CameraData
{
    vec4 position;
    mat4 projview;
} camera_data;

void main()
{
    gl_Position = camera_data.projview * vec4(positions[gl_VertexIndex], 0.0, 1.0);
    fragColor = colors[gl_VertexIndex];
}
