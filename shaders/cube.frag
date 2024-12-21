#version 450

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) out vec4 out_color;

layout(location = 0) in vec3 frag_pos;
layout(location = 1) in vec3 frag_normal;

void main() {

    DirLight light = push_constants.dir_light.data;
    vec3 light_ambient = vec3(0.2, 0.2, 0.2);
    vec3 light_diffuse = vec3(1.0, 1.0, 1.0);
    vec3 light_specular = vec3(1.0, 1.0, 1.0);
    vec3 viewPos = push_constants.camera_data.position.xyz;
    float shininess = 32;
    vec3 cube_color = vec3(1.0, 1.0, 1.0);
    vec3 light_color = vec3(1.0, 1.0, 1.0);

    // ambient
    vec3 ambient = light_ambient * cube_color;

    // diffuse
    vec3 norm = normalize(frag_normal);
    vec3 lightDir = normalize((-light.dir).xyz);
    float diff = max(dot(lightDir, norm), 0.0);
    vec3 diffuse = light_diffuse * diff * cube_color;

    // specular
    vec3 viewDir = normalize(viewPos - frag_pos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), shininess);
    vec3 specular = light_specular * spec * 0.5;
    vec3 result = ambient * 0.0 + diffuse * 1.0 + specular * 1.0;

    out_color = vec4(result, 1.0);
}

