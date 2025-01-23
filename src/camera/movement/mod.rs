pub mod arcball;
pub mod fps;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone)]
    pub struct Flags: u8 {
        const None = 0;
        const Forward = 1 << 0;
        const Backward = 1 << 1;
        const Left = 1 << 2;
        const Right = 1 << 3;
        const Up = 1 << 4;
        const Down = 1 << 5;
    }
}

const SENSITIVITY: f32 = 0.005;
const SPEED: f32 = 0.250;

pub trait Movement {
    fn position(&self) -> glm::Vec4;
    fn compute_matrix(&self) -> glm::Mat4;
    fn update_position(&mut self, movement: Flags);
    fn look_around(&mut self, mouse_dx: f32, mouse_dy: f32);

    fn yaw(&self) -> f32;
    fn pitch(&self) -> f32;

    fn update_ui(&mut self, ui: &imgui::Ui) -> bool;
}

pub fn direction_vector(yaw: f32, pitch: f32) -> glm::Vec3 {
    glm::make_vec3(&[
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    ])
    .normalize()
}
