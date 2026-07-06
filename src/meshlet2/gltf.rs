use crate::{
    assets::gltf_asset,
    meshlet2::{
        build_meshlets,
        gpu::{GeometryInstanceTransform, GlobalGeometryData, Meshlet, MeshletInstance, Vertex},
    },
};

#[derive(Clone)]
pub struct MeshletInfo {
    pub meshlets_offset: u32, // offset into global meshlets array
    pub meshlets_count: u32,
}

struct MeshCacheEntry {
    meshlets_info: std::vec::Vec<MeshletInfo>, // per each primitive of the mesh
}

type MeshCache = std::collections::HashMap<usize, MeshCacheEntry>;
type AssetMeshCache = std::collections::HashMap<std::string::String, MeshCache>;

pub struct Parser {
    pub geometry_data: GlobalGeometryData,
    asset_mesh_cache: AssetMeshCache,
}

pub struct GltfSceneDrawData {
    pub instances_transforms_data: std::vec::Vec<GeometryInstanceTransform>,
    pub meshlet_instances: std::vec::Vec<MeshletInstance>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            geometry_data: GlobalGeometryData {
                vertices: vec![],
                meshlet_vertices: vec![],
                meshlet_triangles: vec![],
                meshlets: vec![],
            },
            asset_mesh_cache: AssetMeshCache::new(),
        }
    }

    pub fn parse(
        &mut self,
        gltf_asset: &gltf_asset::GltfAssetData,
        init_transform: glm::Mat4,
    ) -> std::vec::Vec<GltfSceneDrawData> {
        let mesh_cache = self
            .asset_mesh_cache
            .entry(gltf_asset.path.clone())
            .or_insert(MeshCache::new());

        let global_vertex_buffer = &mut self.geometry_data.vertices;
        let global_meshlets_buffer = &mut self.geometry_data.meshlets;
        let global_meshlets_vertices_buffer = &mut self.geometry_data.meshlet_vertices;
        let global_meshlets_triangles_buffer = &mut self.geometry_data.meshlet_triangles;

        let mut scenes_draw_data: std::vec::Vec<GltfSceneDrawData> = vec![];

        let gltf_parsing_time = std::time::Instant::now();

        let mut node_stack: std::vec::Vec<NodeStackEntry> = vec![];

        struct NodeStackEntry {
            node_index: usize,
            parent_transform: glm::Mat4,
        }

        for scene in &gltf_asset.scenes {
            // TODO push_mut
            let scene_draw_data = {
                scenes_draw_data.push(GltfSceneDrawData {
                    instances_transforms_data: vec![],
                    meshlet_instances: vec![],
                });
                scenes_draw_data.last_mut().unwrap()
            };
            let geometry_instances_transforms = &mut scene_draw_data.instances_transforms_data;
            let meshlet_instances = &mut scene_draw_data.meshlet_instances;

            for node in &scene.nodes {
                node_stack.push(NodeStackEntry {
                    node_index: *node,
                    parent_transform: init_transform,
                });
            }

            while let Some(entry) = node_stack.pop() {
                let (node_index, parent_transform) = { (entry.node_index, entry.parent_transform) };

                let node = &gltf_asset.nodes[node_index];
                let world_transform = parent_transform * node.matrix;

                if let Some(mesh_index) = node.mesh_index {
                    if let Some(mesh_entry) = mesh_cache.get(&mesh_index) {
                        for meshlet_info in &mesh_entry.meshlets_info {
                            geometry_instances_transforms.push(GeometryInstanceTransform {
                                transform: world_transform,
                            });

                            let geometry_transform_index = geometry_instances_transforms.len() - 1;
                            for i in 0..meshlet_info.meshlets_count {
                                meshlet_instances.push(MeshletInstance {
                                    geometry_transform_index: geometry_transform_index as u32,
                                    meshlet_index: meshlet_info.meshlets_offset + i,
                                });
                            }
                        }
                    } else {
                        let mesh = &gltf_asset.meshes[mesh_index];
                        let mut meshlets_info: std::vec::Vec<MeshletInfo> = vec![];
                        for primitive in &mesh.primitives {
                            let vb = &primitive.vertex_buffer;
                            // TODO avoid clone?
                            let ib = match &primitive.index_buffer {
                                gltf_asset::IndexBufferType::U16(items) => {
                                    items.iter().map(|u16val| *u16val as u32).collect()
                                }
                                gltf_asset::IndexBufferType::U32(items) => items.clone(),
                            };

                            let meshlets = build_meshlets::build_meshlets(vb, &ib);

                            let mut meshlet_info = MeshletInfo {
                                meshlets_offset: 0,
                                meshlets_count: 0,
                            };
                            {
                                let meshlets_offset = global_meshlets_buffer.len() as u32;
                                let vertices_offset = global_vertex_buffer.len() as u32;

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

                                // rebase meshlet vertices to global index
                                global_meshlets_vertices_buffer
                                    .extend(meshlets.vertices.iter().map(|v| v + vertices_offset));
                                global_meshlets_triangles_buffer.extend(meshlets.triangles);

                                meshlet_info.meshlets_offset = meshlets_offset;
                                meshlet_info.meshlets_count = meshlets.meshlets.len() as u32;

                                meshlets_info.push(meshlet_info.clone());
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

                            geometry_instances_transforms.push(GeometryInstanceTransform {
                                transform: world_transform,
                            });

                            let geometry_transform_index = geometry_instances_transforms.len() - 1;
                            for i in 0..meshlet_info.meshlets_count {
                                meshlet_instances.push(MeshletInstance {
                                    geometry_transform_index: geometry_transform_index as u32,
                                    meshlet_index: meshlet_info.meshlets_offset + i,
                                })
                            }
                        }

                        mesh_cache.insert(mesh_index, MeshCacheEntry { meshlets_info });
                    }
                }

                for node_index in &node.children {
                    node_stack.push(NodeStackEntry {
                        node_index: *node_index,
                        parent_transform: world_transform,
                    })
                }
            }
        }

        println!("Gltf parsing time: {:?}", gltf_parsing_time.elapsed());

        scenes_draw_data
    }
}

pub fn parse(
    gltf_asset: &gltf_asset::GltfAssetData,
    geometry_data_cache: &mut GlobalGeometryData,
) -> (
    std::vec::Vec<std::vec::Vec<GeometryInstanceTransform>>,
    std::vec::Vec<std::vec::Vec<MeshletInstance>>,
) {
    let global_vertex_buffer = &mut geometry_data_cache.vertices;
    let global_meshlets_buffer = &mut geometry_data_cache.meshlets;
    let global_meshlets_vertices_buffer = &mut geometry_data_cache.meshlet_vertices;
    let global_meshlets_triangles_buffer = &mut geometry_data_cache.meshlet_triangles;
    let mut scene_geometry_instances_transforms: std::vec::Vec<
        std::vec::Vec<GeometryInstanceTransform>,
    > = vec![];
    let mut scene_meshlet_instances: std::vec::Vec<std::vec::Vec<MeshletInstance>> = vec![];

    let gltf_parsing_time = std::time::Instant::now();

    let mut node_stack: std::vec::Vec<NodeStackEntry> = vec![];
    let mut mesh_cache: std::collections::HashMap<usize, MeshCacheEntry> =
        std::collections::HashMap::new();

    struct NodeStackEntry {
        node_index: usize,
        parent_transform: glm::Mat4,
    }

    for scene in &gltf_asset.scenes {
        // TODO push_mut
        let geometry_instances_transforms = {
            scene_geometry_instances_transforms.push(vec![]);
            scene_geometry_instances_transforms.last_mut().unwrap()
        };
        let meshlet_instances = {
            scene_meshlet_instances.push(vec![]);
            scene_meshlet_instances.last_mut().unwrap()
        };
        for node in &scene.nodes {
            node_stack.push(NodeStackEntry {
                node_index: *node,
                parent_transform: glm::Mat4::identity(),
            });
        }

        while let Some(entry) = node_stack.pop() {
            let (node_index, parent_transform) = { (entry.node_index, entry.parent_transform) };

            let node = &gltf_asset.nodes[node_index];
            let world_transform = parent_transform * node.matrix;

            if let Some(mesh_index) = node.mesh_index {
                if let Some(mesh_entry) = mesh_cache.get(&mesh_index) {
                    for meshlet_info in &mesh_entry.meshlets_info {
                        geometry_instances_transforms.push(GeometryInstanceTransform {
                            transform: world_transform,
                        });

                        let geometry_transform_index = geometry_instances_transforms.len() - 1;
                        for i in 0..meshlet_info.meshlets_count {
                            meshlet_instances.push(MeshletInstance {
                                geometry_transform_index: geometry_transform_index as u32,
                                meshlet_index: meshlet_info.meshlets_offset + i,
                            });
                        }
                    }
                } else {
                    let mesh = &gltf_asset.meshes[mesh_index];
                    let mut meshlets_info: std::vec::Vec<MeshletInfo> = vec![];
                    for primitive in &mesh.primitives {
                        let vb = &primitive.vertex_buffer;
                        // TODO avoid clone?
                        let ib = match &primitive.index_buffer {
                            gltf_asset::IndexBufferType::U16(items) => {
                                items.iter().map(|u16val| *u16val as u32).collect()
                            }
                            gltf_asset::IndexBufferType::U32(items) => items.clone(),
                        };

                        let meshlets = build_meshlets::build_meshlets(vb, &ib);

                        let mut meshlet_info = MeshletInfo {
                            meshlets_offset: 0,
                            meshlets_count: 0,
                        };
                        {
                            let meshlets_offset = global_meshlets_buffer.len() as u32;
                            let vertices_offset = global_vertex_buffer.len() as u32;

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

                            // rebase meshlet vertices to global index
                            global_meshlets_vertices_buffer
                                .extend(meshlets.vertices.iter().map(|v| v + vertices_offset));
                            global_meshlets_triangles_buffer.extend(meshlets.triangles);

                            meshlet_info.meshlets_offset = meshlets_offset;
                            meshlet_info.meshlets_count = meshlets.meshlets.len() as u32;

                            meshlets_info.push(meshlet_info.clone());
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

                        geometry_instances_transforms.push(GeometryInstanceTransform {
                            transform: world_transform,
                        });

                        let geometry_transform_index = geometry_instances_transforms.len() - 1;
                        for i in 0..meshlet_info.meshlets_count {
                            meshlet_instances.push(MeshletInstance {
                                geometry_transform_index: geometry_transform_index as u32,
                                meshlet_index: meshlet_info.meshlets_offset + i,
                            })
                        }
                    }

                    mesh_cache.insert(mesh_index, MeshCacheEntry { meshlets_info });
                }
            }

            for node_index in &node.children {
                node_stack.push(NodeStackEntry {
                    node_index: *node_index,
                    parent_transform: world_transform,
                })
            }
        }
    }

    println!("Gltf parsing time: {:?}", gltf_parsing_time.elapsed());

    (scene_geometry_instances_transforms, scene_meshlet_instances)
}
