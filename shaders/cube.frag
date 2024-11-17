#version 450

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) out vec4 out_color;

layout(location = 0) in vec4 frag_pos;
layout(location = 1) in vec4 frag_normal;

vec3 directional_lights_part(vec3 view_dir);

void main() {

    DirLight light = push_constants.dir_light.data;

    // ambient
    float ambientStrength = 0.1;
    vec3 ambient = (ambientStrength * light.ambient).xyz;

    // diffuse
    vec3 norm = normalize(frag_normal).xyz;
    float diff = max(dot(norm, -light.dir.xyz), 0.0);
    vec3 diffuse = (diff * light.diffuse).xyz;

    // specular
    float specularStrength = 0.1;
    vec3 viewDir = normalize(push_constants.camera_data.position - frag_pos).xyz;
    vec3 reflectDir = reflect(light.dir.xyz, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32);
    vec3 specular = specularStrength * spec * light.specular.xyz;
    vec3 cube_color = vec3(0.0, 0.2, 1.0);
    vec3 result = (ambient + diffuse + specular) * cube_color;

    out_color = vec4(result, 1.0);
}

