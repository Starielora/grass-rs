pub trait GuiSceneNode {
    fn update(self: &mut Self, ui: &imgui::Ui);
}

pub trait GuiCameraNode {
    fn update(self: &mut Self, ui: &imgui::Ui);
}
