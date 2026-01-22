#extension GL_EXT_buffer_reference : require
#extension GL_EXT_nonuniform_qualifier: require
#extension GL_EXT_shader_8bit_storage : require

layout(set = 0, binding = 0) uniform samplerCube skybox_tx[];
layout(set = 0, binding = 1) uniform sampler2D depth_textures[];

layout(buffer_reference) readonly buffer CameraData {
  vec4 position;
  mat4 projview;
};

layout(buffer_reference) readonly buffer MeshData {
  mat4 model_matrix;
};

struct DirLight {
  vec4 dir;
  vec4 color;
};

layout(buffer_reference) readonly buffer DirLightBuffer {
  DirLight data;
};

// lol
layout(buffer_reference) readonly buffer SkyboxData {
  uint current_texture_id;
};

struct Vertex {
  float vx, vy, vz;
  float nx, ny, nz;
  float tx, ty;
};

layout(buffer_reference) readonly buffer MeshVertexData {
  Vertex vertices[];
};

layout(buffer_reference) readonly buffer MeshletTriangles {
  uint8_t meshlet_triangles[];
};

layout(buffer_reference) readonly buffer MeshletVertices {
  uint meshlet_vertices[];
};

struct Meshlet {
  uint vertex_offset;
  uint triangle_offset;
  uint vertex_count;
  uint triangle_count;
};

layout(buffer_reference) readonly buffer MeshletData {
  Meshlet meshlets[];
};

layout(push_constant) uniform constants
{
  MeshData mesh_data;
  CameraData camera_data;
  CameraData dir_light_camera_data;
  DirLightBuffer dir_light;
  SkyboxData skybox_data;
  MeshletData meshlet_data;
  MeshVertexData mesh_vertex_data;
  MeshletVertices meshlet_vertices;
  MeshletTriangles meshlet_triangles;
  uint depth_sampler_index;
} push_constants;
