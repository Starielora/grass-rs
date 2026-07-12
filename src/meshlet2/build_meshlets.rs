pub fn build_meshlets(
    vertices: &std::vec::Vec<f32>,
    indices: &std::vec::Vec<u32>,
) -> (meshopt::Meshlets, std::vec::Vec<meshopt::Bounds>) {
    let vertices_slice = unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * std::mem::size_of::<f32>(),
        )
    };
    assert!(vertices.len() % 8 == 0);
    // TODO this stride is giga bad, consider using strongly typed vector - needs change during gltf load. Enforce the type there.
    let vertex_adapter =
        meshopt::VertexDataAdapter::new(vertices_slice, std::mem::size_of::<f32>() * 8, 0)
            .expect("Failed to create vertex adapter");

    // TODO revise max vertices and triangle count - fix in shaders as well
    // TODO use cone weight, when implementing cone culling
    let meshopt_meshlets =
        meshopt::build_meshlets(indices.as_slice(), &vertex_adapter, 64, 124, 0.5);

    // TODO repack the data, to reduce size. Cone data perhaps unnecessary
    let mut meshlets_bounds = vec![];
    for meshlet in meshopt_meshlets.iter() {
        let meshlet_bounds = meshopt::compute_meshlet_bounds(meshlet, &vertex_adapter);
        meshlets_bounds.push(meshlet_bounds);
    }

    (meshopt_meshlets, meshlets_bounds)
}

pub fn simplify(
    indices: &std::vec::Vec<u32>,
    vertices: &std::vec::Vec<f32>,
    target_count: usize,
    target_error: f32,
) -> std::vec::Vec<u32> {
    let vertices_slice = unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * std::mem::size_of::<f32>(),
        )
    };
    let vertex_adapter =
        meshopt::VertexDataAdapter::new(vertices_slice, std::mem::size_of::<f32>() * 8, 0)
            .expect("Failed to create vertex adapter");

    meshopt::simplify(
        indices.as_slice(),
        &vertex_adapter,
        target_count,
        target_error,
        meshopt::SimplifyOptions::None,
        Option::None,
    )
}

pub fn compute_sphere_bounds(vertices: &std::vec::Vec<f32>) -> meshopt::Sphere {
    let vertices_slice = unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * std::mem::size_of::<f32>(),
        )
    };
    assert!(vertices.len() % 8 == 0);
    let position_data_adapter = meshopt::PositionDataAdapter {
        data: vertices_slice,
        position_count: vertices.len() / 8,
        position_stride: std::mem::size_of::<f32>() * 8,
        position_offset: 0,
    };

    let sphere = meshopt::compute_sphere_bounds(position_data_adapter, None);
    sphere
}
