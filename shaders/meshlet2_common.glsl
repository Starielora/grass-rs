#include "camera_data.glsl"
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require
#extension GL_EXT_shader_8bit_storage : require

#include "geometry.glsl"

layout(push_constant) uniform constants
{
    CameraDataBuf view_camera;
    CameraDataBuf cull_camera;
    VertexBuf vertices;
    MeshletsVerticesBuf meshlets_vertices;
    MeshletsTrianglesBuf meshlets_triangles;
    MeshesBuf meshes;
    MeshletsBuf meshlets;
    MeshInstanceBuf mesh_instances;
    MeshletInstanceBuf meshlet_instances;
    VisibleMeshletInstancesBuf visible_meshlet_instances;
    VisibleMeshletInstancesCountBuf visible_meshlet_instances_count;
    uint mesh_instances_count;
} push_constants;


layout(constant_id = 0) const uint SUBGROUP_SIZE = 1;
