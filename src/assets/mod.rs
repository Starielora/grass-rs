pub(super) mod asset;
pub(super) mod mesh;
pub(super) mod node;
pub(super) mod primitive;
pub(super) mod scene;

pub use asset::Asset;

use crate::vkutils;
use ash::vk;
use gltf::{self, accessor::DataType};

fn accessor_data_type_to_size(accessor_data_type: gltf::accessor::DataType) -> usize {
    match accessor_data_type {
        DataType::I8 => std::mem::size_of::<i8>(),
        DataType::U8 => std::mem::size_of::<u8>(),
        DataType::I16 => std::mem::size_of::<i16>(),
        DataType::U16 => std::mem::size_of::<u16>(),
        DataType::U32 => std::mem::size_of::<u32>(),
        DataType::F32 => std::mem::size_of::<f32>(),
    }
}

fn accessor_type_to_components_count(accessor_type: gltf::accessor::Dimensions) -> usize {
    match accessor_type {
        gltf::accessor::Dimensions::Scalar => 1,
        gltf::accessor::Dimensions::Vec2 => 2,
        gltf::accessor::Dimensions::Vec3 => 3,
        gltf::accessor::Dimensions::Vec4 => 4,
        gltf::accessor::Dimensions::Mat2 => 4,
        gltf::accessor::Dimensions::Mat3 => 9,
        gltf::accessor::Dimensions::Mat4 => 16,
    }
}

// TODO can it be done better?
macro_rules! extract_buffer {
    ($type:ty, $document:ident, $buffer_data:ident, $accessor_index:ident) => {{
        let accessor = $document
            .accessors()
            .find(|accessor| accessor.index() == $accessor_index)
            .expect("Failed to find accessor");
        let buffer_view = accessor.view().expect("Buffer view not found");

        let accessor_view_offset = accessor.offset();
        let accessor_elements_count = accessor.count();
        let accesor_total_data_size = accessor_elements_count
            * accessor_data_type_to_size(accessor.data_type())
            * accessor_type_to_components_count(accessor.dimensions());

        let buffer_index = buffer_view.buffer().index();
        let buffer_offset = buffer_view.offset() + accessor_view_offset;
        let buffer_size = accesor_total_data_size;
        // let buffer_stride = buffer_view.stride();

        // TODO make this return vectors, matrices etc.
        let chunks = $buffer_data[buffer_index].0.as_slice()
            [buffer_offset..buffer_offset + buffer_size]
            .chunks_exact(std::mem::size_of::<$type>());
        let mut buffer: std::vec::Vec<$type> = vec![];
        for chunk in chunks {
            let val =
                <$type>::from_le_bytes(chunk[0..std::mem::size_of::<$type>()].try_into().unwrap());
            buffer.push(val);
        }

        buffer
    }};
}

pub enum IndexBufferType {
    U16(std::vec::Vec<u16>),
    U32(std::vec::Vec<u32>),
}

fn extract_index_buffer(
    data_type: DataType,
    document: &gltf::Document,
    buffer_data: &Vec<gltf::buffer::Data>,
    accessor_index: usize,
) -> IndexBufferType {
    return match data_type {
        DataType::I8 => todo!(),
        DataType::U8 => todo!(),
        DataType::I16 => todo!(),
        DataType::U16 => {
            IndexBufferType::U16(extract_buffer!(u16, document, buffer_data, accessor_index))
        }
        DataType::U32 => {
            IndexBufferType::U32(extract_buffer!(u32, document, buffer_data, accessor_index))
        }
        DataType::F32 => todo!(),
    };
}

fn get_data_type(document: &gltf::Document, accessor_index: usize) -> gltf::accessor::DataType {
    document
        .accessors()
        .find(|accessor| accessor.index() == accessor_index)
        .expect("Failed to find accessor")
        .data_type()
}

mod internal {
    pub struct Node {
        pub name: Option<std::string::String>,
        pub children: std::vec::Vec<usize>,
        pub matrix: glm::Mat4,
        pub mesh_index: Option<usize>,
    }

    pub struct Primitive {
        pub vertex_buffer: std::vec::Vec<f32>,
        pub index_buffer: super::IndexBufferType,
    }

    pub struct Mesh {
        pub name: Option<std::string::String>,
        pub primitives: std::vec::Vec<self::Primitive>,
    }

    pub struct Scene {
        pub name: Option<std::string::String>,
        pub nodes: std::vec::Vec<usize>,
    }
}

pub fn better_load(path: &str, ctx: &vkutils::context::VulkanContext) -> asset::Asset {
    println!("Loading: {}", path);
    let (gltf_meshes, gltf_nodes, gltf_scenes, default_scene) = load(path);

    let mut meshes: std::vec::Vec<mesh::Mesh> = vec![];
    let mut nodes: std::vec::Vec<node::Node> = vec![];
    let mut scenes: std::vec::Vec<scene::Scene> = vec![];

    for mesh in gltf_meshes {
        let mut primitives = vec![];
        for primitive in mesh.primitives {
            let vertex_buffer = ctx.upload_buffer(
                &primitive.vertex_buffer,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
            let (index_buffer, indices_count, index_type) = match primitive.index_buffer {
                IndexBufferType::U16(items) => (
                    ctx.upload_buffer(&items, vk::BufferUsageFlags::INDEX_BUFFER),
                    items.len(),
                    ash::vk::IndexType::UINT16,
                ),
                IndexBufferType::U32(items) => (
                    ctx.upload_buffer(&items, vk::BufferUsageFlags::INDEX_BUFFER),
                    items.len(),
                    ash::vk::IndexType::UINT32,
                ),
            };

            primitives.push(primitive::Primitive {
                vertex_buffer,
                index_buffer,
                indices_count,
                index_type,
            });
        }

        meshes.push(mesh::Mesh {
            _name: mesh.name,
            primitives,
            per_parent_node_model_buffer: std::collections::HashMap::new(),
        });
    }

    for node in gltf_nodes {
        nodes.push(node::Node::new(&node));
    }

    for scene in gltf_scenes {
        scenes.push(scene::Scene {
            _name: scene.name,
            nodes: scene.nodes,
        })
    }

    asset::Asset::new(&ctx, meshes, nodes, scenes, default_scene)
}

pub fn load(
    path: &str,
) -> (
    std::vec::Vec<internal::Mesh>,
    std::vec::Vec<internal::Node>,
    std::vec::Vec<internal::Scene>,
    Option<usize>,
) {
    let path = std::path::Path::new(path);
    let dir = path.parent().unwrap();

    let gltf = gltf::Gltf::open(path).expect("Failed to open gltf file");
    let document = &gltf.document;
    let buffer_data = gltf::import_buffers(&document, Some(dir), Option::None)
        .expect("Failed to import gltf buffers.");

    let mut meshes = vec![];
    let mut nodes = vec![];
    let mut scenes = vec![];

    for mesh in document.meshes() {
        let mut primitives: std::vec::Vec<internal::Primitive> = vec![];

        for primitive in mesh.primitives() {
            let mut position_accessor: Option<usize> = Option::None;
            let mut normals_accessor: Option<usize> = Option::None;
            let mut texture_coords_accessor: Option<usize> = Option::None;
            let indices_accessor: Option<usize>;

            for attribute in primitive.attributes() {
                let (semantic, accessor) = attribute;
                match semantic {
                    gltf::Semantic::Positions => {
                        position_accessor = Some(accessor.index());
                    }
                    gltf::Semantic::Normals => {
                        normals_accessor = Some(accessor.index());
                    }
                    gltf::Semantic::Tangents => {}
                    gltf::Semantic::Colors(_) => {}
                    gltf::Semantic::TexCoords(_) => {
                        texture_coords_accessor = Some(accessor.index());
                    }
                    gltf::Semantic::Joints(_) => {}
                    gltf::Semantic::Weights(_) => {}
                }
            }
            match primitive.indices() {
                Some(indices) => {
                    indices_accessor = Some(indices.index());
                }
                None => todo!(),
            }

            let position_buffer_component_type =
                get_data_type(&document, position_accessor.unwrap());
            let index_buffer_component_type = get_data_type(&document, indices_accessor.unwrap());
            let normals_buffer_component_type = get_data_type(&document, normals_accessor.unwrap());

            assert!(position_buffer_component_type == DataType::F32);
            assert!(normals_buffer_component_type == DataType::F32);

            let position_accessor = position_accessor.unwrap();
            // TODO extract with stride, use variants
            let position_buffer: Vec<glm::Vec3> =
                extract_buffer!(f32, document, buffer_data, position_accessor)
                    .chunks_exact(3)
                    .map(glm::make_vec3)
                    .collect();

            let index_buffer = extract_index_buffer(
                index_buffer_component_type,
                &document,
                &buffer_data,
                indices_accessor.unwrap(),
            );

            let normals_accessor = normals_accessor.unwrap();
            let normals_buffer: Vec<glm::Vec3> =
                extract_buffer!(f32, document, buffer_data, normals_accessor)
                    .chunks_exact(3)
                    .map(glm::make_vec3)
                    .collect();

            let texture_coords_buffer =
                if let Some(texture_coords_accessor) = texture_coords_accessor {
                    extract_buffer!(f32, document, buffer_data, texture_coords_accessor)
                        .chunks_exact(2)
                        .map(glm::make_vec2)
                        .collect()
                } else {
                    // TODO o kurwa co tu sie dzieje, prosze nie rob tak
                    let mut v: std::vec::Vec<glm::Vec2> = vec![];
                    for _ in 0..position_buffer.len() {
                        v.push(glm::make_vec2(&[0.0, 0.0]));
                    }
                    v
                };

            // assert!(texture_coords_buffer.len() == position_buffer.len());
            assert!(position_buffer.len() == normals_buffer.len());

            // TODO maybe map + collect
            let mut combined_buffer: Vec<f32> = Vec::new();

            for ((pos, norm), tx) in std::iter::zip(
                std::iter::zip(position_buffer, normals_buffer),
                texture_coords_buffer,
            ) {
                combined_buffer.push(pos.x);
                combined_buffer.push(pos.y);
                combined_buffer.push(pos.z);
                combined_buffer.push(norm.x);
                combined_buffer.push(norm.y);
                combined_buffer.push(norm.z);
                combined_buffer.push(tx.x);
                combined_buffer.push(tx.y);
            }

            primitives.push(internal::Primitive {
                vertex_buffer: combined_buffer,
                index_buffer,
            });
        }

        meshes.push(internal::Mesh {
            name: mesh.name().map(|strslice| strslice.to_string()),
            primitives,
        });
    }

    for node in document.nodes() {
        let children = {
            let mut children = vec![];
            for child in node.children() {
                children.push(child.index());
            }
            children
        };

        let matrix = match node.transform() {
            gltf::scene::Transform::Matrix { matrix } => glm::mat4(
                matrix[0][0],
                matrix[1][0],
                matrix[2][0],
                matrix[3][0],
                matrix[0][1],
                matrix[1][1],
                matrix[2][1],
                matrix[3][1],
                matrix[0][2],
                matrix[1][2],
                matrix[2][2],
                matrix[3][2],
                matrix[0][3],
                matrix[1][3],
                matrix[2][3],
                matrix[3][3],
            ),
            gltf::scene::Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => {
                let mut m = glm::Mat4::identity();
                let translation = [translation[0], translation[1], translation[2]];
                m = glm::translate(&m, &glm::make_vec3(&translation));
                let q = glm::quat(rotation[0], rotation[1], rotation[2], rotation[3]);
                m = m * glm::quat_to_mat4(&q);
                m = glm::scale(&m, &glm::make_vec3(&scale));
                m
            }
        };

        let mesh_index = node.mesh().map(|v| v.index());

        nodes.push(internal::Node {
            name: node.name().map(|strslice| strslice.to_string()),
            children,
            matrix,
            mesh_index,
        });
    }

    for scene in document.scenes() {
        scenes.push(internal::Scene {
            name: scene.name().map(|s| s.to_string()),
            nodes: scene.nodes().map(|node| node.index()).collect(),
        });
    }

    (
        meshes,
        nodes,
        scenes,
        document.default_scene().map(|scene| scene.index()),
    )
}
