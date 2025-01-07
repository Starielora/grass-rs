use crate::gui_scene_node;

pub enum TargetRender {
    Scene,
    ShadowMap,
}

pub struct TargetRenderPicker {
    pub target_render: TargetRender,
}

impl gui_scene_node::GuiSceneNode for TargetRenderPicker {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        if ui.tree_node("Target render").is_some() {
            ui.indent();
            if ui.selectable("Scene") {
                self.target_render = TargetRender::Scene;
            }
            if ui.selectable("Shadow map") {
                self.target_render = TargetRender::ShadowMap;
            }
            ui.unindent();
        }
    }
}
