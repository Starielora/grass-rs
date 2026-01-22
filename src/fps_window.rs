pub struct FrameDurations {
    pub cpu: std::time::Duration,
    pub gpu: std::time::Duration,
    pub shadow_map: std::time::Duration,
    pub color_pass: std::time::Duration,
    pub meshlet_pass: std::time::Duration,
    pub ui: std::time::Duration,
}

pub struct FpsWindow {
    cpu: std::vec::Vec<f32>,
    gpu: std::vec::Vec<f32>,
    shadow_map: std::vec::Vec<f32>,
    color_pass: std::vec::Vec<f32>,
    meshlet_pass: std::vec::Vec<f32>,
    ui: std::vec::Vec<f32>,
    current_offset: usize,
}

const VALUES_COUNT: usize = 25;

fn to_ms(d: &std::time::Duration) -> f32 {
    d.as_secs_f32() * 1000.0
}

fn avg(v: &std::vec::Vec<f32>) -> f32 {
    let sum: f32 = v.iter().sum();
    sum / v.len() as f32
}

fn fps(ms: f32) -> f32 {
    1.0 / (ms / 1000.0)
}

fn build_plot(
    ui: &imgui::Ui,
    name: &str,
    avg: f32,
    vals: &[f32],
    offset: usize,
    display_fps: bool,
) {
    let title = if display_fps {
        format!("{} {:.2}", name, fps(avg))
    } else {
        format!("{}", name)
    };
    ui.plot_lines(title, vals)
        .overlay_text(format!("{:.2} ms", avg))
        .values_offset(offset)
        .scale_max(16.6) // 16.6 ms 60fps
        .scale_min(0.0)
        .build();
}

impl FpsWindow {
    pub fn new() -> Self {
        let vals = vec![0.0f32; VALUES_COUNT];
        Self {
            cpu: vals.clone(),
            gpu: vals.clone(),
            shadow_map: vals.clone(),
            color_pass: vals.clone(),
            meshlet_pass: vals.clone(),
            ui: vals.clone(),
            current_offset: 0,
        }
    }

    pub fn build(&mut self, ui: &imgui::Ui, durations: &FrameDurations) {
        self.cpu[self.current_offset] = to_ms(&durations.cpu);
        self.gpu[self.current_offset] = to_ms(&durations.gpu);
        self.shadow_map[self.current_offset] = to_ms(&durations.shadow_map);
        self.color_pass[self.current_offset] = to_ms(&durations.color_pass);
        self.meshlet_pass[self.current_offset] = to_ms(&durations.meshlet_pass);
        self.ui[self.current_offset] = to_ms(&durations.ui);

        self.current_offset = (self.current_offset + 1) % VALUES_COUNT;

        let cpu_avg = avg(&self.cpu);
        let gpu_avg = avg(&self.gpu);
        let shadow_map_avg = avg(&self.shadow_map);
        let color_pass_avg = avg(&self.color_pass);
        let meshlet_pass_avg = avg(&self.meshlet_pass);
        let ui_avg = avg(&self.ui);

        ui.separator();
        build_plot(
            ui,
            "cpu",
            cpu_avg,
            self.cpu.as_slice(),
            self.current_offset,
            true,
        );
        build_plot(
            ui,
            "gpu",
            gpu_avg,
            self.gpu.as_slice(),
            self.current_offset,
            true,
        );
        build_plot(
            ui,
            "shadow map",
            shadow_map_avg,
            self.shadow_map.as_slice(),
            self.current_offset,
            false,
        );
        build_plot(
            ui,
            "color pass",
            color_pass_avg,
            self.color_pass.as_slice(),
            self.current_offset,
            false,
        );
        build_plot(
            ui,
            "meshlet",
            meshlet_pass_avg,
            self.meshlet_pass.as_slice(),
            self.current_offset,
            false,
        );
        build_plot(
            ui,
            "ui",
            ui_avg,
            self.ui.as_slice(),
            self.current_offset,
            false,
        );
        ui.separator();
    }
}
