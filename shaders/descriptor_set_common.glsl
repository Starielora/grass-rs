#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require
#extension GL_EXT_shader_8bit_storage : require

layout(set = 0, binding = 0) uniform samplerCube skybox_tx[];
layout(set = 0, binding = 1) uniform sampler2D depth_textures[];

layout(buffer_reference) readonly buffer CameraDataBuf {
    vec4 position;
    mat4 projview;
};

layout(buffer_reference) readonly buffer TransformBuf {
    mat4 model_matrix;
};
