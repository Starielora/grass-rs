pub fn compute_matrix(pos: &glm::Vec3, dir: &glm::Vec3, up: &glm::Vec3) -> glm::Mat4 {
    let target = pos + dir;

    let mut scale = glm::Mat4::identity();
    scale.m22 = -1.0; // vulkan

    scale * glm::look_at_lh(pos, &target, up)
}

// TODO ths and from_spherical are kinda the same but from different coordinate systems - think
// about names
pub fn compute_matrix_from_angular(pos: &glm::Vec3, yaw: f32, pitch: f32) -> glm::Mat4 {
    let dir = glm::make_vec3(&[
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    ])
    .normalize();

    let right = dir.cross(&glm::make_vec3(&[0.0, 1.0, 0.0])).normalize();
    let up = right.cross(&dir).normalize();

    compute_matrix(pos, &dir, &up)
}

pub fn from_spherical(
    azimuth: f32,
    inclination: f32,
    distance: f32,
    target: glm::Vec3,
) -> (glm::Mat4, glm::Vec3) {
    let mut mat = glm::Mat4::identity();
    mat = glm::rotate(&mat, azimuth, &glm::make_vec3(&[0.0, -1.0, 0.0]));
    mat = glm::rotate(&mat, inclination, &glm::make_vec3(&[0.0, 0.0, 1.0]));

    let dir = (mat * glm::make_vec4(&[-1.0, 0.0, 0.0, 0.0])).normalize();

    let pos = dir.scale(-distance).xyz() + target;
    let dir = dir.scale(-1.0).xyz();
    let up = glm::make_vec3(&[0.0, 1.0, 0.0]);

    let matrix = compute_matrix(&pos, &dir, &up);

    (matrix, pos)
}
