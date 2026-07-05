use ash::vk;
use glm;

use crate::{assets::gltf_asset, vkutils};

pub mod render;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PushConstants {
    pub view_camera: vk::DeviceAddress,
    pub vertices: vk::DeviceAddress,
    pub meshlet_vertices: vk::DeviceAddress,
    pub meshlet_triangles: vk::DeviceAddress,
    pub meshlets: vk::DeviceAddress,
    pub geometry: vk::DeviceAddress,
    pub geometry_instances: vk::DeviceAddress,
    pub meshlet_instances: vk::DeviceAddress,
    pub meshlet_instances_count: u32,
}

impl PushConstants {
    pub fn data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const PushConstants) as *const u8,
                std::mem::size_of::<PushConstants>(),
            )
        }
    }
}

const _: () = assert!(std::mem::size_of::<PushConstants>() <= 128);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pos: glm::Vec3,
    norm: glm::Vec3,
    tx: glm::Vec2,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Meshlet {
    vertex_offset: u32, // offset into meshlet_vertices array (not actual vertex buffer)
    triangle_offset: u32, // offset into meshlet_triangles array
    vertex_count: u32,
    triangle_count: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Geometry {
    vertex_offset: u32,   // offset into global vertex buffer offset
    meshlets_offset: u32, // offset into global meshlets array
    meshlets_count: u32,
}

// NOTE: This is read in shaders through a buffer_reference block (std430), where
// `mat4` has base alignment 16, forcing the struct alignment to 16 and its size to
// 80 bytes. nalgebra-glm's Mat4 is only align-4, so without the explicit align(16)
// the CPU stride would be 68, and every element past index 0 would be misaligned.
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct GeometryInstance {
    transform: glm::Mat4,
    index: u32, // index into Geometry array
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct MeshletInstance {
    geometry_instance_index: u32, // index into the GeometryInstance buffer (keeps transform + Geometry ref)
    meshlet_index: u32,           // global index into the meshlets buffer
}

const _: () = assert!(std::mem::size_of::<GeometryInstance>() == 80);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GeometryDataGPU {
    vertices: vk::DeviceAddress, // actual vertices buffer, array of 8 * f32 a.k.a Vertex
    meshlet_vertices: vk::DeviceAddress, // array of u32, indexing into actual vertex_buffer with Meshlet::vertex_offset + meshlet_vertices[i] over Meshlet::vertex_count -> input for gl_MeshVerticesEXT
    meshlet_triangles: vk::DeviceAddress, // array of u8, indexing into meshlet_vertices with Meshlet::triangle_offset + meshlet_triangles[i] over Meshlet::triangle_count -> input for gl_PrimitiveTriangleIndicesEXT
    meshlets: vk::DeviceAddress,          // array of Meshlet
    geometry: vk::DeviceAddress,          // array of Geometry
}

pub struct GeometryDataCPU {
    pub vertices: std::vec::Vec<Vertex>,
    pub meshlet_vertices: std::vec::Vec<u32>,
    pub meshlet_triangles: std::vec::Vec<u8>,
    pub meshlets: std::vec::Vec<Meshlet>,
    pub geometry: std::vec::Vec<Geometry>,
}

// TODO cleanup all these struct duplicates. Some are probably only local during asset creation
pub struct GeometryDataHandles {
    pub vertices: vkutils::buffer::Buffer,
    pub meshlet_vertices: vkutils::buffer::Buffer,
    pub meshlet_triangles: vkutils::buffer::Buffer,
    pub meshlets: vkutils::buffer::Buffer,
    pub geometry: vkutils::buffer::Buffer,
}

pub struct Asset {
    pub geometry_data_buffers: GeometryDataGPU,
    pub scene_geometry_instances: vkutils::buffer::Buffer,
    pub scene_geometry_instances_count: u32,
    pub scene_meshlet_instances: vkutils::buffer::Buffer,
    pub scene_meshlet_instances_count: u32,
}

impl Asset {
    // TODO split preparing geometry from uploading to GPU?
    // Probably yes, because it will allow me to load many assets into single buffers
    pub fn from_gltf(
        ctx: &vkutils::context::VulkanContext,
        gltf_asset: &gltf_asset::GltfAssetData,
    ) -> (GeometryDataHandles, std::vec::Vec<Self>) {
        struct MeshEntry {
            geometries: std::vec::Vec<u32>, // indices into global geometry buffer
        }

        struct NodeEntry {
            node_index: usize,
            parent_transform: glm::Mat4,
        }

        let mut global_geometry_data = GeometryDataCPU {
            vertices: vec![],
            meshlet_vertices: vec![],
            meshlet_triangles: vec![],
            meshlets: vec![],
            geometry: vec![],
        };

        let global_vertex_buffer = &mut global_geometry_data.vertices;
        let global_geometry_buffer = &mut global_geometry_data.geometry;
        let global_meshlets_buffer = &mut global_geometry_data.meshlets;
        let global_meshlets_vertices_buffer = &mut global_geometry_data.meshlet_vertices;
        let global_meshlets_triangles_buffer = &mut global_geometry_data.meshlet_triangles;
        let mut scene_geometry_instances: std::vec::Vec<std::vec::Vec<GeometryInstance>> = vec![];
        let mut scene_meshlet_instances: std::vec::Vec<std::vec::Vec<MeshletInstance>> = vec![];

        let mut node_stack: std::vec::Vec<NodeEntry> = vec![];
        let mut mesh_entries: std::collections::HashMap<usize, MeshEntry> =
            std::collections::HashMap::new();

        for scene in &gltf_asset.scenes {
            // TODO push_mut
            let geometry_instances = {
                scene_geometry_instances.push(vec![]);
                scene_geometry_instances.last_mut().unwrap()
            };
            let meshlet_instances = {
                scene_meshlet_instances.push(vec![]);
                scene_meshlet_instances.last_mut().unwrap()
            };
            for node in &scene.nodes {
                node_stack.push(NodeEntry {
                    node_index: *node,
                    parent_transform: glm::Mat4::identity(),
                });
            }

            while let Some(entry) = node_stack.pop() {
                let (node_index, parent_transform) = { (entry.node_index, entry.parent_transform) };

                let node = &gltf_asset.nodes[node_index];
                let world_transform = parent_transform * node.matrix;

                if let Some(mesh_index) = node.mesh_index {
                    // use cache
                    if let Some(mesh_entry) = mesh_entries.get(&mesh_index) {
                        for geometry_index in &mesh_entry.geometries {
                            geometry_instances.push(GeometryInstance {
                                index: *geometry_index,
                                transform: world_transform,
                            });

                            let geometry_instance_index = geometry_instances.len() - 1;
                            let geometry = &global_geometry_buffer[*geometry_index as usize];
                            for i in 0..geometry.meshlets_count {
                                meshlet_instances.push(MeshletInstance {
                                    geometry_instance_index: geometry_instance_index as u32,
                                    meshlet_index: geometry.meshlets_offset + i,
                                });
                            }
                        }
                    } else {
                        // build meshlets
                        let mesh = &gltf_asset.meshes[mesh_index];
                        let mut mesh_geometries: std::vec::Vec<u32> = vec![];
                        for primitive in &mesh.primitives {
                            let vb = &primitive.vertex_buffer;
                            // TODO avoid clone?
                            let ib = match &primitive.index_buffer {
                                gltf_asset::IndexBufferType::U16(items) => {
                                    items.iter().map(|u16val| *u16val as u32).collect()
                                }
                                gltf_asset::IndexBufferType::U32(items) => items.clone(),
                            };

                            let meshlets = build_meshlets(vb, &ib);

                            let index_in_global_geometry_buffer =
                                global_geometry_buffer.len() as u32;
                            {
                                mesh_geometries.push(index_in_global_geometry_buffer);
                                let meshlets_offset = global_meshlets_buffer.len() as u32;
                                let global_meshlet_vertices_offset =
                                    global_meshlets_vertices_buffer.len() as u32;
                                let global_meshlet_triangles_offset =
                                    global_meshlets_triangles_buffer.len() as u32;

                                for meshlet in &meshlets.meshlets {
                                    global_meshlets_buffer.push(Meshlet {
                                        // rebase offsets to global buffers
                                        vertex_offset: meshlet.vertex_offset
                                            + global_meshlet_vertices_offset,
                                        triangle_offset: meshlet.triangle_offset
                                            + global_meshlet_triangles_offset,
                                        vertex_count: meshlet.vertex_count,
                                        triangle_count: meshlet.triangle_count,
                                    })
                                }

                                global_meshlets_vertices_buffer.extend(meshlets.vertices);
                                global_meshlets_triangles_buffer.extend(meshlets.triangles);

                                global_geometry_buffer.push(Geometry {
                                    vertex_offset: global_vertex_buffer.len() as u32,
                                    meshlets_offset: meshlets_offset, // Fill data. Bake global meshlets buffer offset (rebase from local)
                                    meshlets_count: meshlets.meshlets.len() as u32,
                                });
                            }

                            {
                                // guards for unsafe below
                                const _: () = assert!(
                                    std::mem::size_of::<Vertex>() == 8 * std::mem::size_of::<f32>()
                                );
                                const _: () = assert!(
                                    std::mem::align_of::<Vertex>() == std::mem::align_of::<f32>()
                                );
                                let verts: &[Vertex] = unsafe {
                                    std::slice::from_raw_parts(
                                        primitive.vertex_buffer.as_ptr() as *const Vertex,
                                        primitive.vertex_buffer.len() / 8,
                                    )
                                };

                                global_vertex_buffer.extend_from_slice(verts);
                            }

                            geometry_instances.push(GeometryInstance {
                                index: index_in_global_geometry_buffer,
                                transform: world_transform,
                            });

                            let geometry_instance_index = geometry_instances.len() - 1;
                            let geometry =
                                &global_geometry_buffer[index_in_global_geometry_buffer as usize];
                            for i in 0..geometry.meshlets_count {
                                meshlet_instances.push(MeshletInstance {
                                    geometry_instance_index: geometry_instance_index as u32,
                                    meshlet_index: geometry.meshlets_offset + i,
                                })
                            }
                        }

                        mesh_entries.insert(
                            mesh_index,
                            MeshEntry {
                                geometries: mesh_geometries,
                            },
                        );
                    }
                }

                for node_index in &node.children {
                    node_stack.push(NodeEntry {
                        node_index: *node_index,
                        parent_transform: world_transform,
                    })
                }
            }
        }

        {
            let meshlets_buffer = ctx.upload_buffer(
                global_meshlets_buffer,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let meshlets_vertices_buffer = ctx.upload_buffer(
                global_meshlets_vertices_buffer,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let meshlets_triangles_buffer = ctx.upload_buffer(
                global_meshlets_triangles_buffer,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let vertex_buffer = ctx.upload_buffer(
                global_vertex_buffer,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let geometry_buffer = ctx.upload_buffer(
                global_geometry_buffer,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );
            let mut out: std::vec::Vec<Self> = vec![];
            for (i, geometry_instances) in scene_geometry_instances.iter().enumerate() {
                let meshlet_instances = &scene_meshlet_instances[i];
                let geometry_instances_buf = ctx.upload_buffer(
                    geometry_instances,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );

                let meshlet_instances_buf = ctx.upload_buffer(
                    meshlet_instances,
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );

                out.push(Self {
                    geometry_data_buffers: GeometryDataGPU {
                        vertices: vertex_buffer.device_address.unwrap(),
                        meshlet_vertices: meshlets_vertices_buffer.device_address.unwrap(),
                        meshlet_triangles: meshlets_triangles_buffer.device_address.unwrap(),
                        meshlets: meshlets_buffer.device_address.unwrap(),
                        geometry: geometry_buffer.device_address.unwrap(),
                    },
                    scene_geometry_instances: geometry_instances_buf,
                    scene_geometry_instances_count: geometry_instances.len() as u32,
                    scene_meshlet_instances: meshlet_instances_buf,
                    scene_meshlet_instances_count: meshlet_instances.len() as u32,
                });
            }

            let handles = GeometryDataHandles {
                vertices: vertex_buffer,
                meshlet_vertices: meshlets_vertices_buffer,
                meshlet_triangles: meshlets_triangles_buffer,
                meshlets: meshlets_buffer,
                geometry: geometry_buffer,
            };

            return (handles, out);
        }
    }
}

pub fn build_meshlets(
    vertices: &std::vec::Vec<f32>,
    indices: &std::vec::Vec<u32>,
) -> meshopt::Meshlets {
    let vertices_slice = unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * std::mem::size_of::<f32>(),
        )
    };
    assert!(vertices.len() % 8 == 0);
    const _: () = assert!(std::mem::size_of::<Vertex>() == 8 * std::mem::size_of::<f32>());
    const _: () = assert!(std::mem::align_of::<Vertex>() == std::mem::align_of::<f32>());
    // TODO this stride is giga bad, consider using strongly typed vector - needs change during gltf load. Enforce the type there.
    let vertex_adapter =
        meshopt::VertexDataAdapter::new(vertices_slice, std::mem::size_of::<f32>() * 8, 0)
            .expect("Failed to create vertex adapter");

    // TODO revise max vertices and triangle count - fix in shaders as well
    // TODO use cone weight, when implementing cone culling
    let meshopt_meshlets =
        meshopt::build_meshlets(indices.as_slice(), &vertex_adapter, 64, 124, 0.5);

    meshopt_meshlets
}
