#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

const vec3 pos[4] = vec3[4](
    vec3(-1.0, 0.0,  1.0),
    vec3(-1.0, 0.0, -1.0),
    vec3( 1.0, 0.0,  1.0),
    vec3( 1.0, 0.0, -1.0)
);

const int indices[6] = int[6](
    0, 1, 2, 1, 3, 2
);

layout (location = 0) out vec3 world_pos;
layout (location = 1) out float grid_size;

void main()
{
    CameraData camera_data = push_constants.camera_data;

    grid_size = 100.0;
    int idx = indices[gl_VertexIndex];
    vec3 position = pos[idx] * grid_size;

    position.x += camera_data.position.x;
    position.z += camera_data.position.z;
    position.y = 0.0;

    gl_Position = camera_data.projview * vec4(position, 1.0);
    world_pos = position;
}
