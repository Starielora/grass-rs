#version 460 core

layout(set = 0, binding = 0) uniform CameraData
{
    vec4 position;
    mat4 projview;
} camera_data;

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec3 world_pos;
layout(location = 1) in float grid_size;

#define PI 3.1415
#define AXIS
#define ARROWS

void main()
{
    vec3 dir = world_pos - camera_data.position.xyz;

    float distance_to_camera = length(dir.xz);

    float x_size = 1.;
    float z_size = x_size;
    float thickness = 0.01;
    float x_step = abs(sin(x_size * world_pos.x*PI));
    float z_step = abs(sin(z_size * world_pos.z*PI));

    float linecount = 2.0 * x_size;
    float blendregion = 2.8;

    vec2 dF = fwidth(world_pos.xz) * linecount;
    float valueX = 1.0 - smoothstep(dF.s * thickness, dF.s * (thickness + blendregion), x_step);
    float valueY = 1.0 - smoothstep(dF.t * thickness, dF.t * (thickness + blendregion), z_step);
    vec3 vertical = vec3(valueX);
    vec3 horizontal = vec3(valueY);
    float bloom = smoothstep(0.0, 1., distance_to_camera/100.);

    vec3 color = max(vertical + bloom, horizontal + bloom);
    color *= vec3(0.25,0.25,0.25);

    const float alpha = (1. - pow(distance_to_camera/grid_size, 3.0)) * length(color);
    out_color = vec4(color, alpha);

    #ifdef AXIS
    const vec3 red = vec3(1.0, 0.0, 0.0);
    const vec3 blue = vec3(0.0, 0.0, 1.0);
    float length_yz = length(world_pos * vec3(0., 1., 1.));
    float length_xy = length(world_pos * vec3(1., 1., 0.));

    // poor man's AA
    // good enough for now...
    // TODO make axis fade away in distance and maybe research how to make it better overall
    const float line_width = mix(0.04, 0.32, distance_to_camera/grid_size);
    const float aa_width = mix(0.002, 0.32, distance_to_camera/grid_size);
    vec4 red_line = vec4(red, smoothstep(line_width, line_width - aa_width, length_yz));
    vec4 blue_line = vec4(blue, smoothstep(line_width, line_width - aa_width, length_xy));
    if (length_yz < line_width)
    {
        const float s = smoothstep(line_width - aa_width, line_width, length_yz);
        vec4 redblue_lines = mix(red_line, blue_line, s);
        if (length_xy < line_width)
        {
            out_color = redblue_lines;
        }
        else
        {
            out_color = mix(redblue_lines, out_color, s);
        }
    }
    else if (length_xy < line_width)
    {
        const float s = smoothstep(line_width - aa_width, line_width, length_xy);
        out_color = mix(blue_line, out_color, s);
    }

    #ifdef ARROWS
    if (world_pos.x > 0.75 && world_pos.x < 1.0 && world_pos.z > -0.25 && world_pos.z < 0.25)
    {
        float z1 = world_pos.x - 1.f;
        float z2 = -world_pos.x + 1.f;
        float delta1 = abs(z1) - abs(world_pos.z);
        float delta2 = abs(z2) - abs(world_pos.z);
        if (delta1 > 0. && delta2 > 0.)
            out_color = vec4(1., 0., 0., 1.);
    }
    else if (world_pos.z > 0.75 && world_pos.z < 1.0 && world_pos.x > -0.25 && world_pos.x < 0.25)
    {
        float x1 = world_pos.z - 1.f;
        float x2 = -world_pos.z + 1.f;
        float delta1 = abs(x1) - abs(world_pos.x);
        float delta2 = abs(x2) - abs(world_pos.x);
        if (delta1 > 0. && delta2 > 0.)
            out_color = vec4(0., 0., 1., 1.);
    }
    #endif
    #endif
}
