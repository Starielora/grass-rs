#include "camera_data.glsl"
#include "geometry.glsl"

struct Sphere {
    vec3 center;
    float radius;
};

struct TaskPayload {
    Sphere spheres[64];
};

layout(push_constant) uniform constant
{
    CameraDataBuf view_camera;
    MeshesBuf meshes;
    MeshletsBuf meshlets;
    MeshInstanceBuf mesh_instances;
    VisibleMeshletInstancesBuf visible_meshlet_instances;
    VisibleMeshletInstancesCountBuf visible_meshlet_instances_count;
    uint mesh_instances_count;
} push_constants;
