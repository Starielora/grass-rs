use app::App;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod bindless_descriptor_set;
mod camera;
mod cube;
mod dir_light;
mod drawable;
mod grid;
mod gui;
mod gui_scene_node;
mod push_constants;
mod skybox;
mod vkutils;

extern crate nalgebra_glm as glm;

fn main() {
    let event_loop = EventLoop::new().expect("Error creating event loop.");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();

    event_loop.run_app(&mut app).expect("App failed");
}
