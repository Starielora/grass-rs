use ash::vk;

use crate::{stats, vkutils};

pub struct Gui2 {
    platform: imgui_winit_support::WinitPlatform,
    imguictx: imgui::Context,
    imgui_renderer: imgui_rs_vulkan_renderer::Renderer,
    window: std::rc::Rc<winit::window::Window>,
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
        let ev: winit::event::Event<_> = winit::event::Event::WindowEvent {
            window_id,
            event: event.clone(),
        };

        self.platform
            .handle_event::<()>(self.imguictx.io_mut(), &self.window, &ev);
    }

    pub fn prepare_frame(self: &mut Self, stats: &[stats::StatRow]) {
        let ui = self.imguictx.frame();

        ui.window("Stats")
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .title_bar(false)
            .always_auto_resize(true)
            .build(|| build_stats_table(ui, stats));

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

const BUDGET_OK: [f32; 4] = [0.4, 1.0, 0.4, 1.0]; // < 60 fps budget
const BUDGET_WARN: [f32; 4] = [1.0, 1.0, 0.4, 1.0]; // < 30 fps budget
const BUDGET_OVER: [f32; 4] = [1.0, 0.4, 0.4, 1.0];

fn frame_budget_color(ms: f64) -> [f32; 4] {
    if ms < 1000.0 / 60.0 {
        BUDGET_OK
    } else if ms < 1000.0 / 30.0 {
        BUDGET_WARN
    } else {
        BUDGET_OVER
    }
}

fn build_stats_table(ui: &imgui::Ui, stats: &[stats::StatRow]) {
    let Some(_table) = ui.begin_table_with_flags(
        "stats-table",
        5,
        imgui::TableFlags::SIZING_FIXED_FIT | imgui::TableFlags::ROW_BG,
    ) else {
        return;
    };

    ui.table_setup_column("pass");
    ui.table_setup_column("avg");
    ui.table_setup_column("min");
    ui.table_setup_column("max");
    ui.table_setup_column("% frame");
    ui.table_headers_row();

    // avg (ns) of the enclosing time scope per depth; None for non-time rows
    let mut parents: std::vec::Vec<Option<f64>> = std::vec::Vec::new();

    for row in stats {
        let avg = row.avg();
        let avg_ns = match avg {
            stats::StatValue::Time(d) => Some(d.as_nanos() as f64),
            _ => None,
        };

        parents.truncate(row.depth as usize);
        let parent_ns = parents.last().copied().flatten();
        parents.push(avg_ns);

        ui.table_next_row();
        ui.table_next_column();
        ui.text(format!("{}{}", "  ".repeat(row.depth as usize), row.name));

        // budget coloring only for top-level time rows (frame totals)
        let color = avg_ns
            .filter(|_| row.depth == 0)
            .map(|ns| frame_budget_color(ns / 1e6));

        // default font is monospace, so right-align by padding
        ui.table_next_column();
        let avg_text = format!("{:>9}", avg.format());
        match color {
            Some(c) => ui.text_colored(c, avg_text),
            None => ui.text(avg_text),
        }

        ui.table_next_column();
        ui.text(format!("{:>9}", row.min().format()));
        ui.table_next_column();
        ui.text(format!("{:>9}", row.max().format()));

        ui.table_next_column();
        match (avg_ns, parent_ns) {
            (Some(ns), _) if row.depth == 0 && ns > 0.0 => {
                ui.text_colored(frame_budget_color(ns / 1e6), format!("{:>7.0} fps", 1e9 / ns));
            }
            (Some(ns), Some(parent)) if parent > 0.0 => {
                ui.text(format!("{:>7.1}%", 100.0 * ns / parent));
            }
            _ => {}
        }
    }
}
