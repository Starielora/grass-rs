use cgmath::*;

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
    pos: Vector3<f32>,
    dir: Vector3<f32>,
    up: Vector3<f32>,

    speed: f32,
    sensitivity: f32,

    yaw: Deg<f32>,
    pitch: Deg<f32>,

    movement: MovementFlags,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CameraData {
    pub pos: Vector4<f32>,
    pub projview: Matrix4<f32>,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            pos: vec3(0.0, 1.0, 5.0),
            dir: vec3(0.0, 0.0, -1.0),
            up: vec3(0.0, 1.0, 0.0),
            speed: 0.125,
            sensitivity: 0.1,
            yaw: Deg(90.0 as f32),
            pitch: Deg(0.0 as f32),
            movement: MovementFlags::None,
        }
    }

    pub fn update_pos(&mut self) {
        let mut delta = vec3(0.0, 0.0, 0.0);

        if self.movement.contains(MovementFlags::Left) {
            delta += self.dir.cross(self.up).normalize();
        }

        if self.movement.contains(MovementFlags::Right) {
            delta -= self.dir.cross(self.up).normalize();
        }

        if self.movement.contains(MovementFlags::Forward) {
            delta -= self.dir;
        }

        if self.movement.contains(MovementFlags::Backward) {
            delta += self.dir;
        }

        if self.movement.contains(MovementFlags::Up) {
            delta += vec3(0.0, 1.0, 0.0);
        }

        if self.movement.contains(MovementFlags::Down) {
            delta += vec3(0.0, -1.0, 0.0);
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

        self.yaw += Deg(delta_x);
        self.pitch += Deg(delta_y);

        if self.pitch > Deg(89.0) {
            self.pitch = Deg(89.0);
        }

        if self.pitch < Deg(-89.0) {
            self.pitch = Deg(-89.0);
        }

        let mut dir = Vector3::new(0.0, 0.0, 0.0);

        dir.x = self.yaw.cos() * self.pitch.cos();
        dir.y = self.pitch.sin();
        dir.z = self.yaw.sin() * self.pitch.cos();

        self.dir = dir.normalize();
        let right = self.dir.cross(vec3(0.0, 1.0, 0.0)).normalize();
        self.up = right.cross(self.dir).normalize();
    }

    pub fn projection(w: f32, h: f32) -> Matrix4<f32> {
        cgmath::perspective(Deg(45.0), w / h, 0.01, 100.0)
    }

    pub fn view(&self) -> Matrix4<f32> {
        let pos = Point3::from_vec(self.pos);
        let target = Point3::from_vec(self.pos + self.dir);

        cgmath::Transform::look_at_lh(pos, target, self.up)
    }

    pub fn get_projection_view(&self, w: f32, h: f32) -> Matrix4<f32> {
        let mut scale = Matrix4::<f32>::identity();
        scale[1][1] = -1.0;
        scale * Camera::projection(w, h) * self.view()
    }
}
