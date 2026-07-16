use crate::vkutils;
use ash::vk;

// GPU timing with named begin/end scopes instead of raw query indices.
// Scopes may nest; nesting depth is recorded for indented display.
//
// Note: single query pool, so results must be read only after the frame's GPU
// work completed (the render loop currently waits idle every frame). Once
// frames in flight become real, turn the pool into a per-frame ring — the API
// stays the same.
pub struct GpuProfiler {
    vk: ash::Device,
    query_pool: vk::QueryPool,
    timestamp_period: f64,
    max_scopes: u32,
    scopes: std::vec::Vec<Scope>,
    open: std::vec::Vec<u32>, // stack of open scope indices
}

struct Scope {
    name: &'static str,
    depth: u8,
}

#[must_use = "close the scope with GpuProfiler::end"]
pub struct ScopeToken(u32);

impl GpuProfiler {
    pub fn new(ctx: &vkutils::context::VulkanContext, max_scopes: u32) -> Self {
        let query_pool_create_info = vk::QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(2 * max_scopes);

        let query_pool = unsafe {
            ctx.device
                .create_query_pool(&query_pool_create_info, None)
                .expect("Failed to create query pool")
        };

        Self {
            vk: ctx.device.clone(),
            query_pool,
            timestamp_period: ctx.physical_device.props.limits.timestamp_period as f64,
            max_scopes,
            scopes: std::vec::Vec::new(),
            open: std::vec::Vec::new(),
        }
    }

    /// Call once per frame, right after begin_command_buffer and before any begin().
    pub fn begin_frame(&mut self, command_buffer: vk::CommandBuffer) {
        self.scopes.clear();
        self.open.clear();
        unsafe {
            self.vk
                .cmd_reset_query_pool(command_buffer, self.query_pool, 0, 2 * self.max_scopes);
        }
    }

    pub fn begin(&mut self, command_buffer: vk::CommandBuffer, name: &'static str) -> ScopeToken {
        let index: u32 = self.scopes.len().try_into().unwrap();
        assert!(index < self.max_scopes, "GpuProfiler: max_scopes exceeded");

        self.scopes.push(Scope {
            name,
            depth: self.open.len() as u8,
        });
        self.open.push(index);

        unsafe {
            // BOTTOM_OF_PIPE for both begin and end: the begin timestamp is written
            // only once all previously submitted work drained, so a scope doesn't
            // absorb the cost of the passes before it.
            self.vk.cmd_write_timestamp(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                self.query_pool,
                2 * index,
            );
        }

        ScopeToken(index)
    }

    pub fn end(&mut self, command_buffer: vk::CommandBuffer, token: ScopeToken) {
        let expected = self
            .open
            .pop()
            .expect("GpuProfiler: end() without open scope");
        assert_eq!(
            expected, token.0,
            "GpuProfiler: scopes must close in LIFO order"
        );

        unsafe {
            self.vk.cmd_write_timestamp(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                self.query_pool,
                2 * token.0 + 1,
            );
        }
    }

    /// Read back the frame's timings. Call after the GPU finished the frame.
    /// Returns (name, depth, duration) per scope, in recording order.
    pub fn collect(&mut self) -> std::vec::Vec<(&'static str, u8, std::time::Duration)> {
        assert!(
            self.open.is_empty(),
            "GpuProfiler: unclosed scope at collect()"
        );

        if self.scopes.is_empty() {
            return std::vec::Vec::new();
        }

        let mut raw: std::vec::Vec<u64> = vec![0; 2 * self.scopes.len()];
        unsafe {
            self.vk
                .get_query_pool_results(
                    self.query_pool,
                    0,
                    raw.as_mut_slice(),
                    vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                )
                .expect("Failed to get query results");
        }

        self.scopes
            .iter()
            .enumerate()
            .map(|(i, scope)| {
                let ticks = raw[2 * i + 1].saturating_sub(raw[2 * i]);
                let nanos = (ticks as f64 * self.timestamp_period) as u64;
                (
                    scope.name,
                    scope.depth,
                    std::time::Duration::from_nanos(nanos),
                )
            })
            .collect()
    }
}

impl std::ops::Drop for GpuProfiler {
    fn drop(&mut self) {
        unsafe { self.vk.destroy_query_pool(self.query_pool, None) };
    }
}
