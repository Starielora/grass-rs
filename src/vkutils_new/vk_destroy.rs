// Rust's member destruction order + drop is a bit troublesome when working with Vulkan.
// To allow for a bit more control I'm introducing this trait.

pub trait VkDestroy {
    fn vk_destroy(&self);
}
