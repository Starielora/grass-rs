#include "camera_data.glsl"

#include "bindless_layout.glsl"

layout(push_constant) uniform constants
{
    CameraDataBuf view_camera;
    uint current_texture_id;
} push_constants;
