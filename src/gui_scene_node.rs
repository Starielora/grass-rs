pub trait GuiSceneNode {
    fn update(self: &mut Self, ui: &imgui::Ui);
}
