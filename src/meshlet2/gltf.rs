use crate::{
    assets::gltf_asset,
    meshlet2::{
        build_meshlets,
        gpu::{GeometryInstanceTransform, Meshlet, MeshletInstance, Vertex},
    },
};

pub struct GlobalGeometryData {
    pub vertices: std::vec::Vec<Vertex>,
    pub meshlet_vertices: std::vec::Vec<u32>,
    pub meshlet_triangles: std::vec::Vec<u8>,
    pub meshlets: std::vec::Vec<Meshlet>,
    pub meshlets_info: std::vec::Vec<MeshletInfo>,
}

pub struct MeshletInfo {
    pub meshlets_offset: u32, // offset into global meshlets array
    pub meshlets_count: u32,
}

pub fn parse(
    gltf_asset: &gltf_asset::GltfAssetData,
    geometry_data_cache: &mut GlobalGeometryData,
) -> (
    std::vec::Vec<std::vec::Vec<GeometryInstanceTransform>>,
    std::vec::Vec<std::vec::Vec<MeshletInstance>>,
) {
    let global_vertex_buffer = &mut geometry_data_cache.vertices;
    let global_meshlets_info_buffer = &mut geometry_data_cache.meshlets_info;
    let global_meshlets_buffer = &mut geometry_data_cache.meshlets;
    let global_meshlets_vertices_buffer = &mut geometry_data_cache.meshlet_vertices;
    let global_meshlets_triangles_buffer = &mut geometry_data_cache.meshlet_triangles;
    let mut scene_geometry_instances_transforms: std::vec::Vec<
        std::vec::Vec<GeometryInstanceTransform>,
    > = vec![];
    let mut scene_meshlet_instances: std::vec::Vec<std::vec::Vec<MeshletInstance>> = vec![];

    let gltf_parsing_time = std::time::Instant::now();

    let mut node_stack: std::vec::Vec<NodeEntry> = vec![];
    let mut mesh_entries: std::collections::HashMap<usize, MeshEntry> =
        std::collections::HashMap::new();

    struct MeshEntry {
        transforms_indices: std::vec::Vec<u32>, // indices into global transforms buffer
    }

    struct NodeEntry {
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
                    for transform_index in &mesh_entry.transforms_indices {
                        geometry_instances_transforms.push(GeometryInstanceTransform {
                            transform: world_transform,
                        });

                        let geometry_transform_index = geometry_instances_transforms.len() - 1;
                        let meshlet_info = &global_meshlets_info_buffer[*transform_index as usize];
                        for i in 0..meshlet_info.meshlets_count {
                            meshlet_instances.push(MeshletInstance {
                                geometry_transform_index: geometry_transform_index as u32,
                                meshlet_index: meshlet_info.meshlets_offset + i,
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

                        let meshlets = build_meshlets::build_meshlets(vb, &ib);

                        let index_in_global_geometry_buffer =
                            global_meshlets_info_buffer.len() as u32;
                        {
                            mesh_geometries.push(index_in_global_geometry_buffer);

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

                            global_meshlets_info_buffer.push(MeshletInfo {
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

                        geometry_instances_transforms.push(GeometryInstanceTransform {
                            transform: world_transform,
                        });

                        let geometry_transform_index = geometry_instances_transforms.len() - 1;
                        let meshlet_info =
                            &global_meshlets_info_buffer[index_in_global_geometry_buffer as usize];
                        for i in 0..meshlet_info.meshlets_count {
                            meshlet_instances.push(MeshletInstance {
                                geometry_transform_index: geometry_transform_index as u32,
                                meshlet_index: meshlet_info.meshlets_offset + i,
                            })
                        }
                    }

                    mesh_entries.insert(
                        mesh_index,
                        MeshEntry {
                            transforms_indices: mesh_geometries,
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

    println!("Gltf parsing time: {:?}", gltf_parsing_time.elapsed());

    (scene_geometry_instances_transforms, scene_meshlet_instances)
}
