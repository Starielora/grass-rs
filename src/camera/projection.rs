pub mod perspective {

    // Vec1 instead of simply f32 for easier imgui integration, which requires array ref
    #[derive(Debug, Clone)]
    pub struct Properties {
        pub(super) aspect: f32,
        pub(super) fov: glm::Vec1,
        pub(super) near: glm::Vec1,
        pub(super) far: glm::Vec1,
    }

    impl Properties {
        pub fn new(w: f32, h: f32, fov: f32, near: f32, far: f32) -> Self {
            Self {
                aspect: w / h,
                fov: glm::Vec1::new(fov),
                near: glm::Vec1::new(near),
                far: glm::Vec1::new(far),
            }
        }
    }
}

pub mod orthtographic {

    #[derive(Debug, Clone)]
    pub struct Properties {
        pub(super) left: f32,
        pub(super) right: f32,
        pub(super) bottom: f32,
        pub(super) top: f32,
        pub(super) near: f32,
        pub(super) far: f32,
        pub(super) scale: [f32; 1],
    }

    impl Properties {
        pub fn new(w: f32, h: f32, scale: f32) -> Self {
            let left = w / -100.0;
            let right = w / 100.0;
            let bottom = h / -100.0;
            let top = h / 100.0;
            let near = -100.0;
            let far = 100.0;
            Self {
                left,
                right,
                bottom,
                top,
                near,
                far,
                scale: [scale],
            }
        }
    }
}

pub enum Projection {
    Perspective(perspective::Properties),
    Orthographic(orthtographic::Properties),
}

impl Projection {
    pub fn update_ui(&mut self, ui: &imgui::Ui) -> bool {
        ui.indent();
        let changed = match self {
            Projection::Perspective(props) => {
                let mut changed = [false, false, false];
                changed[0] = imgui::Drag::new("fov")
                    .range(0.0, 100.0)
                    .speed(0.25)
                    .build_array(ui, &mut props.fov.data.0[0]);
                changed[1] = imgui::Drag::new("near")
                    .range(0.01, 100.0)
                    .speed(0.25)
                    .build_array(ui, &mut props.near.data.0[0]);
                changed[2] = imgui::Drag::new("far")
                    .range(0.0, 500.0)
                    .speed(0.25)
                    .build_array(ui, &mut props.far.data.0[0]);

                changed.contains(&true)
            }
            Projection::Orthographic(props) => imgui::Drag::new("distance")
                .range(0.0, 500.0)
                .speed(0.25)
                .build_array(ui, &mut props.scale),
        };
        ui.unindent();
        changed
    }

    pub fn compute_matrix(&self) -> glm::Mat4 {
        match self {
            Projection::Perspective(props) => glm::perspective(
                props.aspect,
                props.fov.x.to_radians(),
                props.near.x,
                props.far.x,
            ),
            Projection::Orthographic(props) => {
                let scale = props.scale[0];
                glm::ortho_zo(
                    scale * props.left,
                    scale * props.right,
                    scale * props.bottom,
                    scale * props.top,
                    scale * props.near,
                    scale * props.far,
                )
            }
        }
    }
}
