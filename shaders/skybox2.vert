#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "skybox2_common.glsl"

const vec3 corners[8] = vec3[](
        vec3(1.0, 1.0, -1.0),
        vec3(1.0, -1.0, -1.0),
        vec3(1.0, 1.0, 1.0),
        vec3(1.0, -1.0, 1.0),
        vec3(-1.0, 1.0, -1.0),
        vec3(-1.0, -1.0, -1.0),
        vec3(-1.0, 1.0, 1.0),
        vec3(-1.0, -1.0, 1.0)
    );

const int indices[36] = int[](
        0, 4, 6, 0, 6, 2,
        3, 2, 6, 3, 6, 7,
        7, 6, 4, 7, 4, 5,
        5, 1, 3, 5, 3, 7,
        1, 0, 2, 1, 2, 3,
        5, 4, 0, 5, 0, 1
    );

layout(location = 0) out vec3 out_uvw;

void main()
{
    vec3 pos = corners[indices[gl_VertexIndex]];
    out_uvw = pos;
    vec4 vertex = vec4(pos * 200.0, 1.0);
    gl_Position = push_constants.view_camera.projview
            * (vertex + push_constants.view_camera.position);
}
