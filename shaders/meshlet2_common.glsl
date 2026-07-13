#include "camera_data.glsl"
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require
#extension GL_EXT_shader_8bit_storage : require

#include "geometry.glsl"

layout(buffer_reference) buffer DrawCounterBuf {
    uint count;
};

layout(push_constant) uniform constants
{
    CameraDataBuf view_camera;
    VertexBuf vertices;
    MeshletsVerticesBuf meshlets_vertices;
    MeshletsTrianglesBuf meshlets_triangles;
    MeshesBuf meshes;
    MeshletsBuf meshlets;
    MeshInstanceBuf mesh_instances;
    MeshletInstanceBuf meshlet_instances;
    TaskDispatchesBuf task_dispatches;
    MeshletInstancesDrawsBuf meshlet_instances_draws;
    DrawCounterBuf draw_counter;
    uint mesh_instances_count;
    uint meshlet_instances_count;
    uint draws_count;
} push_constants;


layout(constant_id = 0) const uint SUBGROUP_SIZE = 1;
