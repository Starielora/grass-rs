#extension GL_GOOGLE_include_directive : enable
#include "descriptor_set_common.glsl"

struct DirLight {
    vec4 dir;
    vec4 color;
};

layout(buffer_reference) readonly buffer DirLightBuf {
    DirLight data;
};

layout(buffer_reference) readonly buffer SkyboxBuf {
    uint current_texture_id;
};

// Array of per-instance transforms, indexed by instance index.
layout(buffer_reference) readonly buffer TraditionalInstanceBuf {
    TransformBuf transforms[];
};

// Per-draw-call offset into the instance buffer, indexed by gl_DrawID.
layout(buffer_reference) readonly buffer TraditionalOffsetBuf {
    uint offset[];
};

layout(buffer_reference, std430) readonly buffer FrustumColorBuf {
    vec4 planes_color;
    vec4 edges_color;
};

layout(push_constant) uniform constants
{
    TransformBuf mesh_transform; // currently unused at runtime
    CameraDataBuf view_camera;
    CameraDataBuf ctrl_camera;
    CameraDataBuf cull_camera;
    CameraDataBuf dir_light_camera;
    DirLightBuf dir_light;
    SkyboxBuf skybox;
    TraditionalInstanceBuf instances;
    TraditionalOffsetBuf instance_offsets;
    uint depth_sampler_index;
    FrustumColorBuf frustum_colors;
} push_constants;
