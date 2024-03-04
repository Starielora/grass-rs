use cgmath::*;

#[derive(Debug)]
pub struct Camera {
    pos: Vector3<f32>,
    dir: Vector3<f32>,
    up: Vector3<f32>,

    speed: f32,
    sensitivity: f32,

    yaw: f32,
    pitch: f32,

    move_left: bool,
    move_right: bool,
    move_fw: bool,
    move_bw: bool,
    move_up: bool,
    move_down: bool,
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
            pos: vec3(0.0, 0.0, 5.0),
            dir: vec3(0.0, 0.0, -1.0),
            up: vec3(0.0, 1.0, 0.0),
            speed: 0.25,
            sensitivity: 0.1,
            yaw: -90.0,
            pitch: 0.0,
            // todo bitflag
            move_left: false,
            move_bw: false,
            move_fw: false,
            move_right: false,
            move_up: false,
            move_down: false,
        }
    }

    pub fn update_pos(&mut self) {
        let mut delta = vec3(0.0, 0.0, 0.0);

        if self.move_left {
            delta -= self.speed * self.dir.cross(self.up).normalize();
        }

        if self.move_right {
            delta += self.speed * self.dir.cross(self.up).normalize();
        }

        if self.move_fw {
            delta += self.speed * self.dir;
        }

        if self.move_bw {
            delta -= self.speed * self.dir;
        }

        if self.move_up {
            delta += vec3(0.0, self.speed, 0.0);
        }

        if self.move_down {
            delta += vec3(0.0, -self.speed, 0.0);
        }

        self.pos += self.speed * delta;
    }

    pub fn set_move_forward(&mut self, toggle: bool) {
        self.move_fw = toggle;
    }

    pub fn set_move_backward(&mut self, toggle: bool) {
        self.move_bw = toggle;
    }

    pub fn set_move_left(&mut self, toggle: bool) {
        self.move_left = toggle;
    }

    pub fn set_move_right(&mut self, toggle: bool) {
        self.move_right = toggle;
    }

    pub fn set_move_up(&mut self, toggle: bool) {
        self.move_up = toggle;
    }

    pub fn set_move_down(&mut self, toggle: bool) {
        self.move_down = toggle;
    }

    pub fn look_around(&mut self, mut delta_x: f32, mut delta_y: f32) {
        delta_x *= self.sensitivity;
        delta_y *= self.sensitivity;

        self.yaw += delta_x;
        self.pitch += delta_y;

        let mut dir = Vector3::new(0.0, 0.0, 0.0);

        dir.x = Deg(self.yaw).cos() * Deg(self.pitch).cos();
        dir.y = Deg(self.pitch).sin();
        dir.z = Deg(self.yaw).sin() * Deg(self.pitch).cos();

        self.dir = dir.normalize();
        let right = dir.cross(vec3(0.0, 1.0, 0.0)).normalize();
        self.up = right.cross(self.dir).normalize();
    }

    pub fn projection(w: f32, h: f32) -> Matrix4<f32> {
        cgmath::perspective(Deg(45.0), w / h, 0.01, 100.0)
    }

    pub fn view(&self) -> Matrix4<f32> {
        let eye = Point3::from_vec(self.pos + self.dir);
        let center = Point3::from_vec(self.pos);

        cgmath::Transform::look_at_lh(eye, center, self.up)
    }

    pub fn get_projection_view(&self, w: f32, h: f32) -> Matrix4<f32> {
        Camera::projection(w, h) * self.view()
    }
}
