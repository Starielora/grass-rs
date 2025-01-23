extern crate nalgebra_glm as glm;
use crate::gui_scene_node::GuiCameraNode;
use movement::Movement;

pub mod movement;
pub mod projection;
pub mod view;

struct GuiData {
    // TODO could be enums, but I'm not even using these values
    projection_selection: i32,
    movement_selection: i32,
}

pub struct Camera {
    movement_flags: movement::Flags,

    perspective_projection_props: projection::perspective::Properties,
    orthographic_projection_props: projection::orthtographic::Properties,
    current_projection: projection::Projection,

    movement: std::boxed::Box<dyn movement::Movement>,

    projection_matrix: glm::Mat4,
    view_matrix: glm::Mat4,

    gui_data: GuiData,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GPUCameraData {
    pub pos: glm::Vec4,
    pub projview: glm::Mat4,
}

impl Camera {
    pub fn new(width: f32, height: f32) -> Camera {
        let opts_perspective =
            projection::perspective::Properties::new(width, height, 45.0, 0.01, 500.0);
        let opts_ortho = projection::orthtographic::Properties::new(width, height, 1.0);
        let current_projection = projection::Projection::Perspective(opts_perspective.clone());
        let projection_matrix = current_projection.compute_matrix();
        let fps_movement = movement::fps::FPS::new(
            glm::make_vec3(&[0.0, 1.0, 5.0]),
            glm::make_vec3(&[0.0, 0.0, -1.0]),
        );
        let view_matrix = fps_movement.compute_matrix();
        Camera {
            movement_flags: movement::Flags::None,
            perspective_projection_props: opts_perspective,
            orthographic_projection_props: opts_ortho,
            current_projection,
            projection_matrix,
            movement: Box::new(fps_movement),
            view_matrix,
            gui_data: GuiData {
                projection_selection: 0,
                movement_selection: 0,
            },
        }
    }

    pub fn update_pos(&mut self) {
        self.movement.update_position(self.movement_flags.clone());
        self.view_matrix = self.movement.compute_matrix();
    }

    pub fn set_move_forward(&mut self, toggle: bool) {
        self.movement_flags.set(movement::Flags::Forward, toggle);
    }

    pub fn set_move_backward(&mut self, toggle: bool) {
        self.movement_flags.set(movement::Flags::Backward, toggle);
    }

    pub fn set_move_left(&mut self, toggle: bool) {
        self.movement_flags.set(movement::Flags::Left, toggle);
    }

    pub fn set_move_right(&mut self, toggle: bool) {
        self.movement_flags.set(movement::Flags::Right, toggle);
    }

    pub fn set_move_up(&mut self, toggle: bool) {
        self.movement_flags.set(movement::Flags::Up, toggle);
    }

    pub fn set_move_down(&mut self, toggle: bool) {
        self.movement_flags.set(movement::Flags::Down, toggle);
    }

    pub fn look_around(&mut self, delta_x: f32, delta_y: f32) {
        self.movement.look_around(delta_x, delta_y);
        self.view_matrix = self.movement.compute_matrix();
    }

    pub fn get_projection_view(&self) -> glm::Mat4 {
        self.projection_matrix * self.view_matrix
    }

    pub fn pos(&self) -> glm::Vec4 {
        self.movement.position()
    }

    fn cache_current_projection_props(&mut self) {
        match &self.current_projection {
            projection::Projection::Perspective(properties) => {
                self.perspective_projection_props = properties.clone()
            }
            projection::Projection::Orthographic(properties) => {
                self.orthographic_projection_props = properties.clone()
            }
        }
    }
}

fn arcball_from_fps(
    fps: &std::boxed::Box<dyn movement::Movement>,
) -> std::boxed::Box<dyn movement::Movement> {
    let r = 1.0;
    let direction = movement::direction_vector(fps.yaw(), fps.pitch()).scale(r);
    let arcball_center = fps.position().xyz() - direction;
    let camera_pos = direction;

    let inclination = (camera_pos.y / r).asin();
    let azimuth = camera_pos.z.signum()
        * (camera_pos.x / (camera_pos.x.powi(2) + camera_pos.z.powi(2)).sqrt()).acos();
    let target = arcball_center;

    std::boxed::Box::new(movement::arcball::Arcball::new2(
        azimuth,
        inclination,
        r,
        target,
    ))
}

fn fps_from_arcball(
    arcball: &std::boxed::Box<dyn movement::Movement>,
) -> std::boxed::Box<dyn movement::Movement> {
    std::boxed::Box::new(movement::fps::FPS::new_from_angles(
        arcball.position().xyz(),
        arcball.yaw(),
        arcball.pitch(),
    ))
}

impl GuiCameraNode for Camera {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        let pos = self.movement.position();
        let dir = movement::direction_vector(self.movement.yaw(), self.movement.pitch());
        let _ = self.movement.compute_matrix();

        ui.separator();
        ui.indent();
        ui.text(format!("pos {:.2}, {:.2}, {:.2}", pos.x, pos.y, pos.z));
        ui.text(format!("dir {:.2}, {:.2}, {:.2}", dir.x, dir.y, dir.z));
        ui.text(format!("yaw {:.2} deg", self.movement.yaw().to_degrees()));
        ui.text(format!(
            "pitch {:.2} deg",
            self.movement.pitch().to_degrees()
        ));
        ui.unindent();
        ui.separator();

        // TODO this whole part looks non-rust, but I'm so tired with this refactor that I
        // don't care. Maybe I'll revisit this in the future.
        ui.text("Projection");
        let perspective_chosen =
            ui.radio_button("perspective", &mut self.gui_data.projection_selection, 0);
        ui.same_line();
        let ortho_chosen =
            ui.radio_button("orthographic", &mut self.gui_data.projection_selection, 1);

        if perspective_chosen {
            self.cache_current_projection_props();
            self.current_projection =
                projection::Projection::Perspective(self.perspective_projection_props.clone());
            self.projection_matrix = self.current_projection.compute_matrix();
        }

        if ortho_chosen {
            self.cache_current_projection_props();
            self.current_projection =
                projection::Projection::Orthographic(self.orthographic_projection_props.clone());
            self.projection_matrix = self.current_projection.compute_matrix();
        }

        let any_value_changed = self.current_projection.update_ui(&ui);
        if any_value_changed {
            self.projection_matrix = self.current_projection.compute_matrix();
        }

        ui.separator();

        ui.text("Movement");
        let fps_chosen = ui.radio_button("FPS", &mut self.gui_data.movement_selection, 0);
        ui.same_line();
        let arcball_chosen = ui.radio_button("Arcball", &mut self.gui_data.movement_selection, 1);

        if fps_chosen {
            self.movement = fps_from_arcball(&self.movement);
            self.view_matrix = self.movement.compute_matrix();
        }

        if arcball_chosen {
            self.movement = arcball_from_fps(&self.movement);
            self.view_matrix = self.movement.compute_matrix();
        }

        let any_value_changed = self.movement.update_ui(ui);
        if any_value_changed {
            self.view_matrix = self.movement.compute_matrix();
        }

        ui.separator();
    }
}
