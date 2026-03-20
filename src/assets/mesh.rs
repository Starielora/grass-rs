use super::meshlet::Meshlet;
use super::primitive::FVFCombinedPrimitives;
use super::primitive::Primitive;

pub enum Primitives {
    FixedFunctionVertexPrimitives(std::vec::Vec<Primitive>),
    Meshlets(std::vec::Vec<Meshlet>),
    FixedVertexFunctionCombined(FVFCombinedPrimitives),
}

pub struct Mesh {
    pub _name: Option<std::string::String>,
    pub primitives: Primitives,
}
