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

    uint instance_index = gl_InstanceIndex + push_constants.instance_offsets.offset[gl_DrawID];

    mat4 model_matrix = push_constants.instances.transforms[instance_index].model_matrix;
    frag_pos = vec3(model_matrix * vertex);
    // frag_pos = vertex.xyz;
    frag_pos_light_space = push_constants.dir_light_camera.projview * vec4(frag_pos, 1.0);

    frag_normal = mat3(transpose(inverse(model_matrix))) * normal;
    // frag_normal = normal;

    gl_Position = push_constants.camera.projview * vec4(frag_pos, 1.0);
}
