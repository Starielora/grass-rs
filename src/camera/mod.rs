extern crate nalgebra_glm as glm;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug)]
    pub struct MovementFlags: u8 {
        const None = 0;
        const Forward = 1 << 0;
        const Backward = 1 << 1;
        const Left = 1 << 2;
        const Right = 1 << 3;
        const Up = 1 << 4;
        const Down = 1 << 5;
    }
}

#[derive(Debug)]
pub struct Camera {
    pos: glm::Vec3,
    dir: glm::Vec3,
    up: glm::Vec3,

    speed: f32,
    sensitivity: f32,

    yaw: f32,
    pitch: f32,

    movement: MovementFlags,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CameraData {
    pub pos: glm::Vec4,
    pub projview: glm::Mat4,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            pos: glm::make_vec3(&[0.0, 1.0, 5.0]),
            dir: glm::make_vec3(&[0.0, 0.0, -1.0]),
            up: glm::make_vec3(&[0.0, 1.0, 0.0]),
            speed: 0.125,
            sensitivity: 0.1,
            yaw: 90.0 as f32,
            pitch: 0.0 as f32,
            movement: MovementFlags::None,
        }
    }

    pub fn update_pos(&mut self) {
        let mut delta = glm::make_vec3(&[0.0, 0.0, 0.0]);

        if self.movement.contains(MovementFlags::Left) {
            delta += glm::cross(&self.dir, &self.up).normalize();
        }

        if self.movement.contains(MovementFlags::Right) {
            delta -= glm::cross(&self.dir, &self.up).normalize();
        }

        if self.movement.contains(MovementFlags::Forward) {
            delta -= self.dir;
        }

        if self.movement.contains(MovementFlags::Backward) {
            delta += self.dir;
        }

        if self.movement.contains(MovementFlags::Up) {
            delta += glm::make_vec3(&[0.0, 1.0, 0.0]);
        }

        if self.movement.contains(MovementFlags::Down) {
            delta += glm::make_vec3(&[0.0, -1.0, 0.0]);
        }

        self.pos += self.speed * delta;
    }

    pub fn set_move_forward(&mut self, toggle: bool) {
        self.movement.set(MovementFlags::Forward, toggle);
    }

    pub fn set_move_backward(&mut self, toggle: bool) {
        self.movement.set(MovementFlags::Backward, toggle);
    }

    pub fn set_move_left(&mut self, toggle: bool) {
        self.movement.set(MovementFlags::Left, toggle);
    }

    pub fn set_move_right(&mut self, toggle: bool) {
        self.movement.set(MovementFlags::Right, toggle);
    }

    pub fn set_move_up(&mut self, toggle: bool) {
        self.movement.set(MovementFlags::Up, toggle);
    }

    pub fn set_move_down(&mut self, toggle: bool) {
        self.movement.set(MovementFlags::Down, toggle);
    }

    pub fn look_around(&mut self, mut delta_x: f32, mut delta_y: f32) {
        delta_x *= self.sensitivity;
        delta_y *= self.sensitivity;

        self.yaw += delta_x;
        self.pitch += delta_y;

        if self.pitch > 89.0 {
            self.pitch = 89.0;
        }

        if self.pitch < -89.0 {
            self.pitch = -89.0;
        }

        let mut dir = glm::make_vec3(&[0.0, 0.0, 0.0]);

        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();

        dir.x = yaw_rad.cos() * pitch_rad.cos();
        dir.y = pitch_rad.sin();
        dir.z = yaw_rad.sin() * pitch_rad.cos();

        self.dir = dir.normalize();
        let right = self
            .dir
            .cross(&glm::make_vec3(&[0.0, 1.0, 0.0]))
            .normalize();
        self.up = right.cross(&self.dir).normalize();
    }

    pub fn projection(w: f32, h: f32) -> glm::Mat4 {
        glm::perspective(w / h, (45.0 as f32).to_radians(), 0.01, 100.0)
    }

    pub fn view(&self) -> glm::Mat4 {
        let target = self.pos + self.dir;

        glm::look_at_lh(&self.pos, &target, &self.up)
    }

    pub fn get_projection_view(&self, w: f32, h: f32) -> glm::Mat4 {
        let mut scale = glm::Mat4::identity();
        scale.m22 = -1.0;
        scale * Camera::projection(w, h) * self.view()
    }
}
