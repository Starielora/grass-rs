#include "camera_data.glsl"

layout(push_constant) uniform constants
{
    CameraDataBuf view_camera;
} push_constants;
