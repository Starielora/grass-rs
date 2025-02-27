#version 460 core

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 tx;

layout(location = 0) out vec3 frag_pos;
layout(location = 1) out vec3 frag_normal;
layout(location = 2) out vec4 frag_pos_light_space;

void main()
{
    vec4 vertex = vec4(pos, 1.0);

    frag_pos = vec3(push_constants.mesh_data.model_matrix * vertex);
    frag_pos_light_space = push_constants.dir_light_camera_data.projview * vec4(frag_pos, 1.0);

    frag_normal = mat3(transpose(inverse(push_constants.mesh_data.model_matrix))) * normal;

    gl_Position = push_constants.camera_data.projview * vec4(frag_pos, 1.0);
}
