pub struct Node {
    pub _name: Option<std::string::String>,
    pub children: std::vec::Vec<usize>,
    pub matrix: glm::Mat4,
    pub mesh_index: Option<usize>,
}

impl Node {
    pub fn new(node: &super::internal::Node) -> Self {
        Self {
            _name: node.name.clone(),
            children: node.children.clone(),
            matrix: node.matrix,
            mesh_index: node.mesh_index,
        }
    }
}
