use crate::{
    assets::gltf_asset,
    meshlet2::{
        build_meshlets,
        gpu::{GeometryBuildData, Mesh, MeshInstance, Meshlet, MeshletInstance, Vertex},
    },
};

#[derive(Clone)]
pub struct MeshCacheInfo {
    pub meshlets_offset: u32, // offset into global meshlets array
    pub meshlets_count: u32,
    pub global_mesh_index: u32, // index into global mesh array (holding bounding data and meshlets offset and count)
}

struct MeshCacheEntry {
    mesh_cache_info: std::vec::Vec<MeshCacheInfo>, // per each primitive of the mesh
}

type MeshCache = std::collections::HashMap<usize, MeshCacheEntry>;
type AssetMeshCache = std::collections::HashMap<std::string::String, MeshCache>;

pub struct GeometryBuilder {
    pub geometry_data: GeometryBuildData,
    asset_mesh_cache: AssetMeshCache,
}

impl GeometryBuilder {
    pub fn new() -> Self {
        Self {
            geometry_data: GeometryBuildData {
                vertices: vec![],
                meshlet_vertices: vec![],
                meshlet_triangles: vec![],
                meshes: vec![],
                meshlets: vec![],
                mesh_instances: vec![],
                meshlet_instances: vec![],
            },
            asset_mesh_cache: AssetMeshCache::new(),
        }
    }

    pub fn add_instance(
        &mut self,
        gltf_asset: &gltf_asset::GltfAssetData,
        init_transform: glm::Mat4,
        scene: Option<usize>,
    ) {
        let mesh_cache = self
            .asset_mesh_cache
            .entry(gltf_asset.path.clone())
            .or_insert(MeshCache::new());

        let global_vertex_buffer = &mut self.geometry_data.vertices;
        let global_meshlets_buffer = &mut self.geometry_data.meshlets;
        let global_meshes_buffer = &mut self.geometry_data.meshes;
        let global_meshlets_vertices_buffer = &mut self.geometry_data.meshlet_vertices;
        let global_meshlets_triangles_buffer = &mut self.geometry_data.meshlet_triangles;

        let gltf_parsing_time = std::time::Instant::now();

        let mut node_stack: std::vec::Vec<NodeStackEntry> = vec![];

        struct NodeStackEntry {
            node_index: usize,
            parent_transform: glm::Mat4,
        }

        let scene = scene.unwrap_or(gltf_asset.default_scene.unwrap_or(0));
        let scene = &gltf_asset.scenes[scene];

        let mut mesh_instances = vec![];
        let mut meshlet_instances = vec![];

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
                    for mesh_cache_info in &mesh_entry.mesh_cache_info {
                        mesh_instances.push(MeshInstance {
                            transform: world_transform,
                            mesh_index: mesh_cache_info.global_mesh_index,
                            _padding: 0,
                        });

                        let mesh_instance_index = mesh_instances.len() - 1;
                        for i in 0..mesh_cache_info.meshlets_count {
                            meshlet_instances.push(MeshletInstance {
                                mesh_instance_index: mesh_instance_index as u32,
                                meshlet_index: mesh_cache_info.meshlets_offset + i,
                            });
                        }
                    }
                } else {
                    let gltf_mesh = &gltf_asset.meshes[mesh_index];
                    let mut mesh_cache_infos: std::vec::Vec<MeshCacheInfo> = vec![];
                    for primitive in &gltf_mesh.primitives {
                        let vb = &primitive.vertex_buffer;
                        // TODO avoid clone?
                        let ib = match &primitive.index_buffer {
                            gltf_asset::IndexBufferType::U16(items) => {
                                items.iter().map(|u16val| *u16val as u32).collect()
                            }
                            gltf_asset::IndexBufferType::U32(items) => items.clone(),
                        };

                        let (meshlets, meshlets_bounds, sphere) =
                            build_meshlets::build_meshlets(vb, &ib);

                        let global_mesh_index = global_meshes_buffer.len() as u32;

                        let mut mesh_cache_info = MeshCacheInfo {
                            meshlets_offset: 0,
                            meshlets_count: 0,
                            global_mesh_index,
                        };

                        {
                            let meshlets_offset = global_meshlets_buffer.len() as u32;
                            let vertices_offset = global_vertex_buffer.len() as u32;

                            let global_meshlet_vertices_offset =
                                global_meshlets_vertices_buffer.len() as u32;
                            let global_meshlet_triangles_offset =
                                global_meshlets_triangles_buffer.len() as u32;

                            assert!(meshlets.meshlets.len() == meshlets_bounds.len());

                            for (meshlet, bounds) in
                                std::iter::zip(&meshlets.meshlets, &meshlets_bounds)
                            {
                                global_meshlets_buffer.push(Meshlet {
                                    // rebase offsets to global buffers
                                    vertex_offset: meshlet.vertex_offset
                                        + global_meshlet_vertices_offset,
                                    triangle_offset: meshlet.triangle_offset
                                        + global_meshlet_triangles_offset,
                                    vertex_count: meshlet.vertex_count,
                                    triangle_count: meshlet.triangle_count,
                                    bounding_sphere_center: bounds.center.into(),
                                    bounding_sphere_radius: bounds.radius,
                                    cone_apex: bounds.cone_apex.into(),
                                    cone_cutoff: bounds.cone_cutoff,
                                    cone_axis: bounds.cone_axis.into(),
                                    _padding: 0.0f32, // unsure if I'll need the u8 versions of cone axis and cutoff. Definitely not right now.
                                })
                            }

                            // rebase meshlet vertices to global index
                            global_meshlets_vertices_buffer
                                .extend(meshlets.vertices.iter().map(|v| v + vertices_offset));
                            global_meshlets_triangles_buffer.extend(meshlets.triangles);

                            let meshlets_count = meshlets.meshlets.len() as u32;
                            mesh_cache_info.meshlets_offset = meshlets_offset;
                            mesh_cache_info.meshlets_count = meshlets_count;

                            mesh_cache_infos.push(mesh_cache_info.clone());

                            global_meshes_buffer.push(Mesh {
                                meshlets_offset,
                                meshlets_count,
                                _padding: [0.0f32, 0.0f32].into(),
                                bounding_sphere_center: sphere.center.into(),
                                bounding_sphere_radius: sphere.radius,
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

                        mesh_instances.push(MeshInstance {
                            transform: world_transform,
                            mesh_index: global_mesh_index,
                            _padding: 0,
                        });

                        let mesh_instance_index = mesh_instances.len() - 1;
                        for i in 0..mesh_cache_info.meshlets_count {
                            meshlet_instances.push(MeshletInstance {
                                mesh_instance_index: mesh_instance_index as u32,
                                meshlet_index: mesh_cache_info.meshlets_offset + i,
                            })
                        }
                    }

                    mesh_cache.insert(
                        mesh_index,
                        MeshCacheEntry {
                            mesh_cache_info: mesh_cache_infos,
                        },
                    );
                }
            }

            for node_index in &node.children {
                node_stack.push(NodeStackEntry {
                    node_index: *node_index,
                    parent_transform: world_transform,
                })
            }
        }

        for meshlet_instance in &mut meshlet_instances {
            meshlet_instance.mesh_instance_index = meshlet_instance.mesh_instance_index
                + self.geometry_data.mesh_instances.len() as u32;
        }

        self.geometry_data.mesh_instances.extend(&mesh_instances);
        self.geometry_data
            .meshlet_instances
            .extend(&meshlet_instances);

        // println!("Gltf parsing time: {:?}", gltf_parsing_time.elapsed());
    }
}
