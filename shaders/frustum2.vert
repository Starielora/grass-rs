#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "camera_data.glsl"

layout(constant_id = 0) const bool IS_WIREFRAME = false;
layout(location = 0) out vec4 out_color;
layout(push_constant) uniform constants {
    CameraDataBuf view_camera;
    CameraDataBuf cull_camera;
    vec4 edges_color;
    vec4 planes_color;
} push_constants;

const vec3 corners[10] = vec3[10](
        vec3(-1.0, -1.0, 0.0), // 0: near bottom-left
        vec3(1.0, -1.0, 0.0), // 1: near bottom-right
        vec3(1.0, 1.0, 0.0), // 2: near top-right
        vec3(-1.0, 1.0, 0.0), // 3: near top-left
        vec3(-1.0, -1.0, 1.0), // 4: far bottom-left
        vec3(1.0, -1.0, 1.0), // 5: far bottom-right
        vec3(1.0, 1.0, 1.0), // 6: far top-right
        vec3(-1.0, 1.0, 1.0), // 7: far top-left
        vec3(0.0, 0.0, 0.0), // 8: near center
        vec3(0.0, 0.0, 1.0) // 9: far center
    );

// 6 faces x 6 vertices = 36 (TRIANGLE_LIST solid)
const int solid_indices[36] = int[36](
        0, 1, 2, 0, 2, 3, // face 0: near (z=0)
        4, 5, 6, 4, 6, 7, // face 1: far (z=1)
        0, 1, 5, 0, 5, 4, // face 2: bottom (y=-1)
        3, 7, 6, 3, 6, 2, // face 3: top (y=+1)
        0, 4, 7, 0, 7, 3, // face 4: left (x=-1)
        1, 2, 6, 1, 6, 5 // face 5: right (x=+1)
    );

// 12 edges + 1 axis line = 13 lines x 2 vertices = 26 (LINE_LIST wireframe)
const int wire_indices[26] = int[26](
        0, 1, 1, 2, 2, 3, 3, 0, // near face
        4, 5, 5, 6, 6, 7, 7, 4, // far face
        0, 4, 1, 5, 2, 6, 3, 7, // connecting edges
        8, 9 // near-to-far center axis
    );

const vec3 face_normals[6] = vec3[6](
        vec3(0.0, 0.0, -1.0), // near
        vec3(0.0, 0.0, 1.0), // far
        vec3(0.0, -1.0, 0.0), // bottom
        vec3(0.0, 1.0, 0.0), // top
        vec3(-1.0, 0.0, 0.0), // left
        vec3(1.0, 0.0, 0.0) // right
    );

void main()
{
    int corner_idx;
    vec4 color;

    if (IS_WIREFRAME) {
        corner_idx = wire_indices[gl_VertexIndex];
        color = gl_VertexIndex >= 24
            ? vec4(1.0, 1.0, 1.0, 1.0) // axis line: white
            : push_constants.edges_color;
    } else {
        corner_idx = solid_indices[gl_VertexIndex];
        int face_idx = gl_VertexIndex / 6;
        float alpha = face_idx < 2 ? 0.0 : push_constants.planes_color.a;
        color = vec4(push_constants.planes_color.rgb, alpha);
    }

    vec3 ndc_pos = corners[corner_idx];
    mat4 inverse_projview = inverse(push_constants.cull_camera.projview);
    vec4 world_pos = inverse_projview * vec4(ndc_pos, 1.0);
    world_pos /= world_pos.w;
    gl_Position = push_constants.view_camera.projview * world_pos;

    out_color = color;
}
