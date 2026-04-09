use crate::vkutils;
use crate::vkutils::vk_destroy::VkDestroy;

pub struct FVFCombinedPrimitives {
    pub vb: vkutils::buffer::Buffer,
    pub ib: vkutils::buffer::Buffer,
    pub primitive_vertex_offset_in_combined_vertex_buffer: std::vec::Vec<u32>,
    pub primitive_index_count: std::vec::Vec<u32>, // number of indices for a primitive at index
    pub primitive_index_offset_in_combined_index_buffer: std::vec::Vec<u32>,
    pub primitive_parent_node_indices: std::vec::Vec<std::vec::Vec<usize>>,
}

impl std::ops::Drop for FVFCombinedPrimitives {
    fn drop(&mut self) {
        self.vb.vk_destroy();
        self.ib.vk_destroy();
    }
}
