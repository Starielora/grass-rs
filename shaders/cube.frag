#version 450

#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set.glsl"

layout(location = 0) out vec4 out_color;

layout(location = 0) in vec3 frag_pos;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec4 frag_pos_light_space;

float calc_shadow() {
    vec3 proj_coords = frag_pos_light_space.xyz / frag_pos_light_space.w;
    vec2 proj_coords_tx = proj_coords.xy * 0.5 + 0.5;
    float closest_depth = texture(depth_textures[push_constants.depth_sampler_index], proj_coords_tx.st).r;
    float current_depth = proj_coords.z;
    float shadow = current_depth > closest_depth ? 1.0 : 0.0;
    return shadow;
}

void main() {

    DirLight light = push_constants.dir_light.data;
    vec3 light_ambient = vec3(0.2, 0.2, 0.2);
    vec3 light_diffuse = vec3(1.0, 1.0, 1.0);
    vec3 light_specular = vec3(1.0, 1.0, 1.0);
    vec3 viewPos = push_constants.camera_data.position.xyz;
    float shininess = 64;
    vec3 cube_color = vec3(1.0, 1.0, 1.0);
    vec3 light_color = vec3(1.0, 1.0, 1.0);

    // ambient
    vec3 ambient = light_ambient * cube_color;

    // diffuse
    vec3 norm = normalize(frag_normal);
    vec3 lightDir = normalize((-light.dir).xyz);
    float diff = max(dot(lightDir, norm), 0.0);
    vec3 diffuse = light_diffuse * diff * cube_color;

    //specular
    vec3 view_dir = normalize(viewPos - frag_pos);
    vec3 halfwayDir = normalize(lightDir + view_dir);
    float spec = pow(max(dot(norm, halfwayDir), 0.0), shininess);
    vec3 specular = spec * light_specular;

    float shadow = calc_shadow();
    vec3 result = (ambient * 0.0) + (1.0 - shadow) * (diffuse + specular);

    out_color = vec4(result, 1.0);
}

