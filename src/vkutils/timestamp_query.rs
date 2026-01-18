use crate::vkutils;
use ash::vk;

pub struct TimestampQuery {
    query_pool: vk::QueryPool,
    device: ash::Device,

    timestamp_period: f32,
    results: std::vec::Vec<u64>,
}

impl TimestampQuery {
    pub fn new(ctx: &vkutils::context::VulkanContext, count: u32) -> Self {
        let query_pool_create_info = vk::QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(count);

        let query_pool = unsafe {
            ctx.device
                .create_query_pool(&query_pool_create_info, None)
                .expect("Failed to create query pool")
        };

        let mut results: std::vec::Vec<u64> = std::vec::Vec::with_capacity(count as usize);
        results.resize(count as usize, 0);

        Self {
            query_pool,
            device: ctx.device.clone(),
            timestamp_period: ctx.physical_device.props.limits.timestamp_period,
            results,
        }
    }

    pub fn reset(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_reset_query_pool(
                command_buffer,
                self.query_pool,
                0,
                self.results.len() as u32,
            );
        }
    }

    pub fn cmd_write(
        &self,
        query_index: u32,
        stage: vk::PipelineStageFlags,
        command_buffer: vk::CommandBuffer,
    ) {
        unsafe {
            self.device
                .cmd_write_timestamp(command_buffer, stage, self.query_pool, query_index)
        };
    }

    pub fn timestamp_period(&self) -> f32 {
        self.timestamp_period
    }

    pub fn get_results(&mut self) -> &std::vec::Vec<u64> {
        unsafe {
            self.device
                .get_query_pool_results(
                    self.query_pool,
                    0,
                    self.results.as_mut_slice(),
                    vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                )
                .expect("Failed to get query resutls");
        };

        &self.results
    }
}

impl std::ops::Drop for TimestampQuery {
    fn drop(&mut self) {
        unsafe { self.device.destroy_query_pool(self.query_pool, None) };
    }
}
