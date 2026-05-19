#version 450
#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set_meshlet.glsl"

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec4 in_color;
layout(location = 1) in vec3 frag_pos;
layout(location = 2) in vec3 frag_normal;
layout(location = 3) in vec4 frag_pos_light_space;

#define DEBUG 0

void main()
{
    #if DEBUG
    out_color = in_color;
    #else
    DirLight light = push_constants.dir_light.data;
    vec3 light_ambient = vec3(0.2, 0.2, 0.2);
    vec3 light_diffuse = vec3(1.0, 1.0, 1.0);
    vec3 light_specular = vec3(1.0, 1.0, 1.0);
    vec3 viewPos = push_constants.cull_camera.position.xyz;
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

    // TODO shadow
    float shadow = 0.0;
    vec3 result = (ambient * 0.0) + (1.0 - shadow) * (diffuse + specular);

    out_color = vec4(result, 1.0);
    #endif
}
