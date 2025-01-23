#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require

layout(set = 0, binding = 0) uniform samplerCube skybox_tx[];
layout(set = 0, binding = 1) uniform sampler2D depth_textures[];

layout(buffer_reference, std430) readonly buffer CameraData {
    vec4 position;
    mat4 projview;
};

layout(buffer_reference, std430) readonly buffer MeshData {
    mat4 model_matrix;
};

struct DirLight {
    vec4 dir;
    vec4 color;
};

layout(buffer_reference, std430) readonly buffer DirLightBuffer {
    DirLight data;
};

// lol
layout(buffer_reference, std430) readonly buffer SkyboxData {
    uint current_texture_id;
};

layout(push_constant) uniform constants
{
    MeshData mesh_data;
    CameraData camera_data;
    CameraData dir_light_camera_data;
    DirLightBuffer dir_light;
    SkyboxData skybox_data;
    uint depth_sampler_index;
} push_constants;
