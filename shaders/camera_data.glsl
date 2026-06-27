#extension GL_EXT_buffer_reference : require

layout(buffer_reference) readonly buffer CameraDataBuf {
    vec4 position;
    mat4 projview;
    mat4 view;
};
