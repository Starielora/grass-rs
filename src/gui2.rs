use ash::vk;

use crate::{frame_times::FrameTimes, vkutils};

pub struct Gui2 {
    platform: imgui_winit_support::WinitPlatform,
    imguictx: imgui::Context,
    imgui_renderer: imgui_rs_vulkan_renderer::Renderer,
    window: std::rc::Rc<winit::window::Window>,
    last_frame_times: FrameTimes,

    smoothed_gpu_total_ms: f32,
}

impl Gui2 {
    pub fn new(
        window: std::rc::Rc<winit::window::Window>,
        ctx: &vkutils::context::VulkanContext,
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
            last_frame_times: FrameTimes::default(),
            smoothed_gpu_total_ms: 0.0f32,
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

    pub fn set_last_frame_times(&mut self, frame_times: FrameTimes) {
        let x = frame_times.gpu_total().as_secs_f32() * 1000.0;
        self.smoothed_gpu_total_ms += (x - self.smoothed_gpu_total_ms) * 0.05;

        self.last_frame_times = frame_times;
    }

    pub fn prepare_frame(self: &mut Self) {
        let ui = self.imguictx.frame();

        let gpu_total = self.smoothed_gpu_total_ms;
        let gpu_total_fps = 1f32 / (gpu_total / 1000f32);
        ui.window("Stats")
            .size([300.0, 150.0], imgui::Condition::FirstUseEver)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .title_bar(false)
            .build(|| {
                ui.text(format!(
                    "CPU frame time: {}",
                    self.last_frame_times.cpu_total
                ));
                ui.text(format!(
                    "GPU Frame time: {:.2} (fps: {:.0})",
                    gpu_total, gpu_total_fps
                ));
                ui.text(format!(
                    "\tcompute_visible_meshlets: {}",
                    self.last_frame_times.compute_visible_meshlets
                ));
                ui.text(format!(
                    "\tprep_draw_mesh_tasks_command: {}",
                    self.last_frame_times.prep_draw_mesh_tasks_command
                ));
                ui.text(format!(
                    "\tdraw_mesh_tasks_indirect: {}",
                    self.last_frame_times.draw_mesh_tasks
                ));
                ui.text(format!(
                    "\tMSAA resolve: {}",
                    self.last_frame_times.msaa_resolve
                ));
            });

        // let mut show = true;
        // ui.show_demo_window(&mut show);
        // ui.show_metrics_window(&mut show);

        self.platform
            .prepare_frame(self.imguictx.io_mut(), &self.window)
            .expect("Failed to prepare frame.");
    }

    pub fn record(&mut self, command_buffer: vk::CommandBuffer) {
        self.imgui_renderer
            .cmd_draw(command_buffer, self.imguictx.render())
            .expect("Could not draw imgui");
    }
}
