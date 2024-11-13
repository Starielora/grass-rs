#version 450

#define BACK_BOTTOM_LEFT vec3(-0.5f, -0.5f, -0.5f)
#define BACK_BOTTOM_RIGHT vec3(0.5f, -0.5f, -0.5f)
#define BACK_TOP_LEFT vec3(-0.5f, 0.5f, -0.5f)
#define BACK_TOP_RIGHT vec3(0.5f, 0.5f, -0.5f)

#define FRONT_BOTTOM_LEFT vec3(-0.5f, -0.5f, 0.5f)
#define FRONT_BOTTOM_RIGHT vec3(0.5f, -0.5f, 0.5f)
#define FRONT_TOP_LEFT vec3(-0.5f, 0.5f, 0.5f)
#define FRONT_TOP_RIGHT vec3(0.5f, 0.5f, 0.5f)

vec3 vertices[] = {
        BACK_BOTTOM_LEFT,
        BACK_BOTTOM_RIGHT,
        BACK_TOP_LEFT,
        BACK_TOP_RIGHT,
        FRONT_TOP_RIGHT,
        BACK_BOTTOM_RIGHT,
        FRONT_BOTTOM_RIGHT,
        FRONT_BOTTOM_LEFT,
        FRONT_TOP_RIGHT,
        FRONT_TOP_LEFT,
        BACK_TOP_LEFT,
        FRONT_BOTTOM_LEFT,
        BACK_BOTTOM_LEFT,
        BACK_BOTTOM_RIGHT,
    };

layout(push_constant) uniform constants
{
    mat4 cube_model;
} push_constants;

layout(set = 0, binding = 0) uniform CameraData
{
    vec4 position;
    mat4 projview;
} camera_data;

layout(location = 0) out vec4 frag_color;

void main()
{
    gl_Position = camera_data.projview * push_constants.cube_model * vec4(vertices[gl_VertexIndex], 1.);
    frag_color = vec4(vertices[gl_VertexIndex], 1.);
}
