use crate::gui_scene_node;

pub enum TargetRender {
    Scene,
    SceneDepth,
    ShadowMap,
    Meshlet,
}

pub struct TargetRenderPicker {
    pub target_render: TargetRender,
}

impl gui_scene_node::GuiSceneNode for TargetRenderPicker {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        if ui
            .tree_node_config("Target render")
            .opened(true, imgui::Condition::Appearing)
            .push()
            .is_some()
        {
            ui.indent();
            if ui.selectable("Scene") {
                self.target_render = TargetRender::Scene;
            }
            if ui.selectable("Scene depth") {
                self.target_render = TargetRender::SceneDepth;
            }
            if ui.selectable("Shadow map") {
                self.target_render = TargetRender::ShadowMap;
            }
            if ui.selectable("Meshlet") {
                self.target_render = TargetRender::Meshlet;
            }
            ui.unindent();
        }
    }
}
