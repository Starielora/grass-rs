use crate::gui_scene_node::GuiSceneNode;
use crate::vkutils_new;
use ash::vk;

pub struct Gui {
    platform: imgui_winit_support::WinitPlatform,
    imguictx: imgui::Context,
    imgui_renderer: imgui_rs_vulkan_renderer::Renderer,
    window: std::rc::Rc<winit::window::Window>,
    scene_nodes: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>>,
}

impl Gui {
    pub fn new(
        window: std::rc::Rc<winit::window::Window>,
        ctx: &vkutils_new::context::VulkanContext,
        nodes: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>>,
    ) -> Self {
        let mut imguictx = imgui::Context::create();
        imguictx.set_ini_filename(None);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imguictx);
        platform.attach_window(
            imguictx.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Rounded,
        );

        let dynamic_rendering = imgui_rs_vulkan_renderer::DynamicRendering {
            color_attachment_format: ctx.swapchain.surface_format.format,
            depth_attachment_format: Some(ctx.depth_format),
        };

        let imgui_renderer = imgui_rs_vulkan_renderer::Renderer::with_default_allocator(
            &ctx.instance,
            ctx.physical_device.handle,
            ctx.device.clone(),
            ctx.graphics_present_queue,
            ctx.graphics_command_pool.handle,
            dynamic_rendering,
            &mut imguictx,
            Some(imgui_rs_vulkan_renderer::Options {
                in_flight_frames: 2,
                sample_count: vk::SampleCountFlags::TYPE_8,
                ..Default::default()
            }),
        )
        .expect("Could not create imgui renderer");

        Self {
            platform,
            imguictx,
            imgui_renderer,
            window,
            scene_nodes: nodes,
        }
    }

    pub fn update_delta_time(self: &mut Self, delta: std::time::Duration) {
        self.imguictx.io_mut().update_delta_time(delta);
    }

    pub fn handle_winit_window_event(
        self: &mut Self,
        window_id: winit::window::WindowId,
        event: &winit::event::WindowEvent,
    ) {
        // handle_window_event is private so I have to wrap this shit, even though handle_event
        // calls only handle_window_event
        let ev: winit::event::Event<_> = winit::event::Event::WindowEvent {
            window_id,
            event: event.clone(),
        };

        self.platform
            .handle_event::<()>(self.imguictx.io_mut(), &self.window, &ev);
    }

    pub fn prepare_frame(self: &mut Self) {
        let ui = self.imguictx.frame();
        let nodes_iter = self.scene_nodes.iter_mut();

        ui.window("Scene")
            .size([300.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                for node in nodes_iter {
                    node.borrow_mut().update(ui);
                }
            });

        self.platform
            .prepare_frame(self.imguictx.io_mut(), &self.window)
            .expect("Failed to prepare frame.");
    }

    pub fn cmd_draw(&mut self, command_buffer: vk::CommandBuffer) {
        self.imgui_renderer
            .cmd_draw(command_buffer, self.imguictx.render())
            .expect("Could not draw imgui");
    }
}
