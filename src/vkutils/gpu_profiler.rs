use crate::vkutils;
use ash::vk;

pub struct GpuProfiler {
    vk: ash::Device,
    query_pool: vk::QueryPool,
    timestamp_period: f64,
    max_scopes: u32,
    frames: [FrameScopes; vkutils::FRAMES_IN_FLIGHT],
    open: std::vec::Vec<u32>, // stack of open scope indices, within the current slot
    current_slot: usize,
}

#[derive(Default)]
struct FrameScopes {
    scopes: std::vec::Vec<Scope>,
    pending: bool,
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
            .query_count(2 * max_scopes * vkutils::FRAMES_IN_FLIGHT as u32);

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
            frames: std::array::from_fn(|_| FrameScopes::default()),
            open: std::vec::Vec::new(),
            current_slot: 0,
        }
    }

    fn slot_base(&self, slot: usize) -> u32 {
        slot as u32 * 2 * self.max_scopes
    }

    pub fn begin_frame(&mut self, command_buffer: vk::CommandBuffer, slot: usize) {
        self.current_slot = slot;
        self.frames[slot].scopes.clear();
        self.frames[slot].pending = true;
        self.open.clear();

        unsafe {
            self.vk.cmd_reset_query_pool(
                command_buffer,
                self.query_pool,
                self.slot_base(slot),
                2 * self.max_scopes,
            );
        }
    }

    pub fn begin(&mut self, command_buffer: vk::CommandBuffer, name: &'static str) -> ScopeToken {
        let slot = self.current_slot;
        let index: u32 = self.frames[slot].scopes.len().try_into().unwrap();
        assert!(index < self.max_scopes, "GpuProfiler: max_scopes exceeded");

        self.frames[slot].scopes.push(Scope {
            name,
            depth: self.open.len() as u8,
        });
        self.open.push(index);

        unsafe {
            self.vk.cmd_write_timestamp(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                self.query_pool,
                self.slot_base(slot) + 2 * index,
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
                self.slot_base(self.current_slot) + 2 * token.0 + 1,
            );
        }
    }

    pub fn collect(
        &mut self,
        slot: usize,
    ) -> std::vec::Vec<(&'static str, u8, std::time::Duration)> {
        assert!(
            self.open.is_empty(),
            "GpuProfiler: unclosed scope at collect()"
        );

        if !self.frames[slot].pending || self.frames[slot].scopes.is_empty() {
            return std::vec::Vec::new();
        }

        let mut raw: std::vec::Vec<u64> = vec![0; 2 * self.frames[slot].scopes.len()];
        unsafe {
            self.vk
                .get_query_pool_results(
                    self.query_pool,
                    self.slot_base(slot),
                    raw.as_mut_slice(),
                    vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                )
                .expect("Failed to get query results");
        }
        self.frames[slot].pending = false;

        self.frames[slot]
            .scopes
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
