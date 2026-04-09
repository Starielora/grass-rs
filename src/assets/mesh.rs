use super::meshlet::Meshlet;
use super::primitive::FVFCombinedPrimitives;

pub enum Primitives {
    Meshlets(std::vec::Vec<Meshlet>),
    FixedVertexFunctionCombined(FVFCombinedPrimitives),
}

pub struct Mesh {
    pub _name: Option<std::string::String>,
    pub primitives: Primitives,
}
