use crate::vkutils;
use ash::vk;

use crate::drawable;

pub struct Gui {
    platform: imgui_winit_support::WinitPlatform,
    imguictx: imgui::Context,
    imgui_renderer: imgui_rs_vulkan_renderer::Renderer,
    cube_model: std::rc::Rc<std::cell::RefCell<cgmath::Matrix4<f32>>>,
}

impl Gui {
    pub fn new(
        window: &winit::window::Window,
        vkctx: &vkutils::Context,
        model: std::rc::Rc<std::cell::RefCell<cgmath::Matrix4<f32>>>,
    ) -> Self {
        let mut imguictx = imgui::Context::create();
        imguictx.set_ini_filename(None);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imguictx);
        platform.attach_window(
            imguictx.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Rounded,
        );

        let imgui_renderer = imgui_rs_vulkan_renderer::Renderer::with_default_allocator(
            &vkctx.instance,
            vkctx.physical_device,
            vkctx.device.clone(),
            vkctx.present_queue,
            vkctx.command_pool,
            vkctx.render_pass,
            &mut imguictx,
            Some(imgui_rs_vulkan_renderer::Options {
                in_flight_frames: 1,
                sample_count: vk::SampleCountFlags::TYPE_8,
                ..Default::default()
            }),
        )
        .expect("Could not create imgui renderer");

        Self {
            platform,
            imguictx,
            imgui_renderer,
            cube_model: model,
        }
    }

    pub fn update_delta_time(self: &mut Self, delta: std::time::Duration) {
        self.imguictx.io_mut().update_delta_time(delta);
    }

    pub fn handle_winit_window_event(
        self: &mut Self,
        window: &winit::window::Window,
        window_id: winit::window::WindowId,
        event: &winit::event::WindowEvent,
    ) {
        // handle_window_event is private so I have to wrap this shit, even though handle_event
        // calls only handle_window_event
        let ev = winit::event::Event::<()>::WindowEvent {
            window_id,
            event: event.clone(),
        };

        self.platform
            .handle_event(self.imguictx.io_mut(), &window, &ev);
    }

    pub fn prepare_frame(self: &mut Self, window: &winit::window::Window) {
        let ui = self.imguictx.frame();
        ui.window("Hello world")
            .size([300.0, 110.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text_wrapped("Hello world!");
                ui.text_wrapped("こんにちは世界！");
                ui.button("This...is...imgui-rs!");
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });
        let mut matrix = self.cube_model.borrow_mut();
        ui.window("Cube").build(|| {
            ui.slider("Pos X", 0.0 as f32, 5.0 as f32, &mut matrix[3][0]);
        });
        self.platform
            .prepare_frame(self.imguictx.io_mut(), &window)
            .expect("Failed to prepare frame.");
    }
}

impl drawable::Drawable for Gui {
    fn cmd_draw(self: &mut Self, command_buffer: &vk::CommandBuffer) {
        self.imgui_renderer
            .cmd_draw(*command_buffer, self.imguictx.render())
            .expect("Could not draw imgui");
    }
}
