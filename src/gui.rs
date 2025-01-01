use crate::gui_scene_node::GuiSceneNode;
use crate::push_constants::GPUPushConstants;
use crate::vkutils;
use ash::vk;

use crate::drawable;

pub struct Gui {
    platform: imgui_winit_support::WinitPlatform,
    imguictx: imgui::Context,
    imgui_renderer: imgui_rs_vulkan_renderer::Renderer,
}

impl Gui {
    pub fn new(window: &winit::window::Window, vkctx: &vkutils::Context) -> Self {
        let mut imguictx = imgui::Context::create();
        imguictx.set_ini_filename(None);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imguictx);
        platform.attach_window(
            imguictx.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Rounded,
        );

        let dynamic_rendering = imgui_rs_vulkan_renderer::DynamicRendering {
            color_attachment_format: vkctx.swapchain.surface_format.format,
            depth_attachment_format: Some(vkctx.depth_image.format),
        };

        let imgui_renderer = imgui_rs_vulkan_renderer::Renderer::with_default_allocator(
            &vkctx.instance,
            vkctx.physical_device.handle,
            vkctx.device.clone(),
            vkctx.present_queue,
            vkctx.command_pool.handle,
            dynamic_rendering,
            &mut imguictx,
            Some(imgui_rs_vulkan_renderer::Options {
                in_flight_frames: 1,
                sample_count: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            }),
        )
        .expect("Could not create imgui renderer");

        Self {
            platform,
            imguictx,
            imgui_renderer,
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

    pub fn prepare_frame(
        self: &mut Self,
        window: &winit::window::Window,
        scene_nodes: &mut std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>>,
    ) {
        let ui = self.imguictx.frame();

        ui.window("Scene")
            .size([300.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                for node in scene_nodes {
                    node.borrow_mut().update(ui);
                }
            });

        self.platform
            .prepare_frame(self.imguictx.io_mut(), &window)
            .expect("Failed to prepare frame.");
    }
}

impl drawable::Drawable for Gui {
    fn cmd_draw(self: &mut Self, command_buffer: &vk::CommandBuffer, _: &GPUPushConstants) {
        self.imgui_renderer
            .cmd_draw(*command_buffer, self.imguictx.render())
            .expect("Could not draw imgui");
    }
}
