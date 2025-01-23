use super::super::view;
use super::Flags;
use super::Movement;
use super::SENSITIVITY;
use super::SPEED;

pub struct FPS {
    pos: glm::Vec3,
    yaw: f32,   // rad
    pitch: f32, // rad
}

fn approx_equal(a: f32, b: f32) -> bool {
    (a - b).abs() < core::f32::EPSILON
}

fn get_azimuth(v: &glm::Vec3) -> f32 {
    if v.x > 0.0 {
        return (v.z / v.x).atan();
    } else if v.x < 0.0 && v.z >= 0.0 {
        return (v.z / v.x).atan() + glm::pi::<f32>();
    } else if v.x < 0.0 && v.z < 0.0 {
        return (v.z / v.x).atan() - glm::pi::<f32>();
    } else if approx_equal(v.x, 0.0) && v.z > 0.0 {
        return glm::half_pi::<f32>();
    } else if approx_equal(v.x, 0.0) && v.z < 0.0 {
        return -glm::half_pi::<f32>();
    } else {
        panic!("undefined");
    }
}

fn get_inclination(v: &glm::Vec3) -> f32 {
    if v.y > 0.0 {
        return v.xz().magnitude() / v.y;
    } else if v.y < 0.0 {
        return v.xz().magnitude() / v.y + glm::pi::<f32>();
    } else if approx_equal(v.y, 0.0) && v.xz().magnitude() != 0.0 {
        return 0.0;
    } else {
        panic!("undefined woot");
    }
}

impl FPS {
    pub fn new(pos: glm::Vec3, dir: glm::Vec3) -> Self {
        let pitch = get_inclination(&dir);
        let yaw = get_azimuth(&dir.scale(-1.0));
        Self { pos, yaw, pitch }
    }

    pub fn new_from_angles(pos: glm::Vec3, yaw: f32, pitch: f32) -> Self {
        Self { pos, yaw, pitch }
    }
}

impl Movement for FPS {
    fn position(&self) -> glm::Vec4 {
        glm::make_vec4(&[self.pos.x, self.pos.y, self.pos.z, 0.0])
    }

    fn compute_matrix(&self) -> glm::Mat4 {
        view::compute_matrix_from_angular(&self.pos, self.yaw, self.pitch)
    }

    fn update_position(&mut self, movement: Flags) {
        let mut delta = glm::make_vec3(&[0.0, 0.0, 0.0]);
        let dir = super::direction_vector(self.yaw, self.pitch);
        let right = dir.cross(&glm::make_vec3(&[0.0, 1.0, 0.0])).normalize();
        let up = right.cross(&dir).normalize();

        if movement.contains(Flags::Left) {
            delta += glm::cross(&dir, &up).normalize();
        }

        if movement.contains(Flags::Right) {
            delta -= glm::cross(&dir, &up).normalize();
        }

        if movement.contains(Flags::Forward) {
            delta -= dir;
        }

        if movement.contains(Flags::Backward) {
            delta += dir;
        }

        if movement.contains(Flags::Up) {
            delta += glm::make_vec3(&[0.0, 1.0, 0.0]);
        }

        if movement.contains(Flags::Down) {
            delta += glm::make_vec3(&[0.0, -1.0, 0.0]);
        }

        self.pos += SPEED * delta;
    }

    fn look_around(&mut self, mouse_dx: f32, mouse_dy: f32) {
        let mouse_dx = mouse_dx * SENSITIVITY;
        let mouse_dy = mouse_dy * SENSITIVITY;

        self.yaw += mouse_dx;
        self.pitch += mouse_dy;

        self.pitch = self
            .pitch
            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
    }

    fn yaw(&self) -> f32 {
        self.yaw
    }
    fn pitch(&self) -> f32 {
        self.pitch
    }

    fn update_ui(&mut self, _ui: &imgui::Ui) -> bool {
        false
    }
}
