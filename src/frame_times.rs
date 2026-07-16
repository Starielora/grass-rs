#[derive(Default, Copy, Clone)]
pub struct FrameTimes {
    pub cpu_total: u64,
    pub gpu_total: u64,
    pub compute_visible_meshlets: u64,
    pub prep_draw_mesh_tasks_command: u64,
    pub draw_mesh_tasks: u64,
    pub msaa_resolve: u64,
}

impl FrameTimes {
    pub fn cpu_total(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(self.cpu_total)
    }

    pub fn gpu_total(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(self.gpu_total)
    }
}
