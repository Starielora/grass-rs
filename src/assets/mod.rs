pub(super) mod asset;
pub(super) mod gltf_asset;
pub(super) mod mesh;
pub(super) mod meshlet;
pub(super) mod primitive;

pub use asset::Asset;

#[derive(Debug)]
pub enum DrawMode {
    Meshlet,
    Traditional,
}
