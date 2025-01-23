use super::super::view;
use super::Flags;
use super::Movement;
use super::SENSITIVITY;
use super::SPEED;

pub struct Arcball {
    azimuth: f32,
    inclination: f32,
    distance: f32,
    pos: glm::Vec3,
    target: glm::Vec3,
}

impl Arcball {
    pub fn new2(yaw: f32, pitch: f32, distance: f32, target: glm::Vec3) -> Self {
        let azimuth = yaw;
        let inclination = pitch;
        let mut mat = glm::Mat4::identity();
        mat = glm::rotate(&mat, azimuth, &glm::make_vec3(&[0.0, -1.0, 0.0]));
        mat = glm::rotate(&mat, inclination, &glm::make_vec3(&[0.0, 0.0, 1.0]));

        let dir = mat * glm::make_vec4(&[-1.0, 0.0, 0.0, 0.0]);

        let pos = dir.scale(-distance).xyz() + target;
        Self {
            azimuth,
            inclination,
            distance,
            pos,
            target,
        }
    }
}

impl Movement for Arcball {
    fn position(&self) -> glm::Vec4 {
        glm::make_vec4(&[self.pos.x, self.pos.y, self.pos.z, 0.0])
    }

    fn compute_matrix(&self) -> glm::Mat4 {
        view::from_spherical(self.azimuth, self.inclination, self.distance, self.target).0
    }

    fn update_position(&mut self, movement: Flags) {
        let mut delta = glm::make_vec3(&[0.0, 0.0, 0.0]);
        let dir = (self.target - self.pos).normalize();
        let right = dir.cross(&glm::make_vec3(&[0.0, 1.0, 0.0])).normalize();
        let up = right.cross(&dir).normalize();

        if movement.contains(Flags::Left) {
            delta -= glm::cross(&dir, &up).normalize();
        }

        if movement.contains(Flags::Right) {
            delta += glm::cross(&dir, &up).normalize();
        }

        if movement.contains(Flags::Forward) {
            delta += dir;
        }

        if movement.contains(Flags::Backward) {
            delta -= dir;
        }

        if movement.contains(Flags::Up) {
            delta += glm::make_vec3(&[0.0, 1.0, 0.0]);
        }

        if movement.contains(Flags::Down) {
            delta += glm::make_vec3(&[0.0, -1.0, 0.0]);
        }

        self.target += SPEED * delta;
        self.pos += SPEED * delta;
    }

    fn look_around(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.azimuth += SENSITIVITY * mouse_dx;
        self.inclination += SENSITIVITY * mouse_dy;

        self.inclination = self
            .inclination
            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());

        let dir = super::direction_vector(self.azimuth, self.inclination);
        self.pos = dir.scale(self.distance).xyz() + self.target;
    }

    fn yaw(&self) -> f32 {
        self.azimuth
    }
    fn pitch(&self) -> f32 {
        self.inclination
    }

    fn update_ui(&mut self, ui: &imgui::Ui) -> bool {
        let distance_changed = imgui::Drag::new("Distance").build(ui, &mut self.distance);
        let target_changed = imgui::Drag::new("Target").build_array(ui, &mut self.target.data.0[0]);

        return distance_changed || target_changed;
    }
}
