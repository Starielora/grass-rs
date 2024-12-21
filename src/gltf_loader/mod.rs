use gltf::{self, accessor::DataType};

// TODO make this generic
fn extract_f32_buffer(
    document: &gltf::Document,
    buffer_data: &Vec<gltf::buffer::Data>,
    accessor_index: usize,
) -> Vec<f32> {
    let accessor = document
        .accessors()
        .find(|accessor| accessor.index() == accessor_index)
        .expect("Failed to find accessor");
    let buffer_view = accessor.view().expect("Buffer view not found");

    let buffer_index = buffer_view.buffer().index();
    let buffer_offset = buffer_view.offset();
    let buffer_size = buffer_view.length();

    let buffer: Vec<f32> = buffer_data[buffer_index].0.as_slice()
        [buffer_offset..buffer_offset + buffer_size]
        .chunks_exact(std::mem::size_of::<f32>())
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(f32::from_le_bytes)
        .collect();

    buffer
}

fn extract_u16_buffer(
    document: &gltf::Document,
    buffer_data: &Vec<gltf::buffer::Data>,
    accessor_index: usize,
) -> Vec<u16> {
    let accessor = document
        .accessors()
        .find(|accessor| accessor.index() == accessor_index)
        .expect("Failed to find accessor");
    let buffer_view = accessor.view().expect("Buffer view not found");

    let buffer_index = buffer_view.buffer().index();
    let buffer_offset = buffer_view.offset();
    let buffer_size = buffer_view.length();

    let buffer: Vec<u16> = buffer_data[buffer_index].0.as_slice()
        [buffer_offset..buffer_offset + buffer_size]
        .chunks_exact(std::mem::size_of::<u16>())
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(u16::from_le_bytes)
        .collect();

    buffer
}

fn get_data_type(document: &gltf::Document, accessor_index: usize) -> gltf::accessor::DataType {
    document
        .accessors()
        .find(|accessor| accessor.index() == accessor_index)
        .expect("Failed to find accessor")
        .data_type()
}

pub fn load(path: &str) -> (std::vec::Vec<f32>, std::vec::Vec<u16>) {
    let (document, buffer_data, _image_data) =
        gltf::import(path).expect("Failed to load gltf file.");

    let mut position_accessor: Option<usize> = Option::None;
    let mut normals_accessor: Option<usize> = Option::None;
    let mut texture_coords_accessor: Option<usize> = Option::None;
    let mut indices_accessor: Option<usize> = Option::None;

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            for attribute in primitive.attributes() {
                let (semantic, accessor) = attribute;
                match semantic {
                    gltf::Semantic::Positions => {
                        position_accessor = Some(accessor.index());
                    }
                    gltf::Semantic::Normals => {
                        normals_accessor = Some(accessor.index());
                    }
                    gltf::Semantic::Tangents => todo!(),
                    gltf::Semantic::Colors(_) => todo!(),
                    gltf::Semantic::TexCoords(_) => {
                        texture_coords_accessor = Some(accessor.index());
                    }
                    gltf::Semantic::Joints(_) => todo!(),
                    gltf::Semantic::Weights(_) => todo!(),
                }
            }
            match primitive.indices() {
                Some(indices) => {
                    indices_accessor = Some(indices.index());
                }
                None => todo!(),
            }
        }
    }

    let position_buffer_component_type = get_data_type(&document, position_accessor.unwrap());
    let index_buffer_component_type = get_data_type(&document, indices_accessor.unwrap());
    let normals_buffer_component_type = get_data_type(&document, normals_accessor.unwrap());
    let texture_buffer_component_type = get_data_type(&document, texture_coords_accessor.unwrap());

    assert!(position_buffer_component_type == DataType::F32);
    assert!(index_buffer_component_type == DataType::U16);
    assert!(normals_buffer_component_type == DataType::F32);
    assert!(texture_buffer_component_type == DataType::F32);

    let position_buffer: Vec<glm::Vec3> =
        extract_f32_buffer(&document, &buffer_data, position_accessor.unwrap())
            .chunks_exact(3)
            .map(glm::make_vec3)
            .collect();
    let index_buffer = extract_u16_buffer(&document, &buffer_data, indices_accessor.unwrap());
    let normals_buffer: Vec<glm::Vec3> =
        extract_f32_buffer(&document, &buffer_data, normals_accessor.unwrap())
            .chunks_exact(3)
            .map(glm::make_vec3)
            .collect();
    let texture_coords_buffer: Vec<glm::Vec2> =
        extract_f32_buffer(&document, &buffer_data, texture_coords_accessor.unwrap())
            .chunks_exact(2)
            .map(glm::make_vec2)
            .collect();

    assert!(texture_coords_buffer.len() == position_buffer.len());
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

    (combined_buffer, index_buffer)
}
