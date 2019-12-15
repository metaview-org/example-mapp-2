use ammolite_math::*;
use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;

#[mlib::mapp]
pub struct ExampleMapp {
    state: Vec<String>,
}

impl Mapp for ExampleMapp {
    fn new() -> Self {
        Self {
            state: Vec::new(),
        }
    }

    fn test(&mut self, arg: String) -> Vec<String> {
        self.state.push(arg);
        self.state.clone()
    }

    fn get_model_matrices(&mut self, secs_elapsed: f32) -> Vec<Mat4> {
        fn construct_model_matrix(scale: f32, translation: &Vec3, rotation: &Vec3) -> Mat4 {
            Mat4::translation(translation)
                * Mat4::rotation_roll(rotation[2])
                * Mat4::rotation_yaw(rotation[1])
                * Mat4::rotation_pitch(rotation[0])
                * Mat4::scale(scale)
        }

        let matrix = construct_model_matrix(
            1.0,
            &[0.0, 0.0, 2.0].into(),
            &[secs_elapsed.sin() * 0.0 * 1.0, std::f32::consts::PI + secs_elapsed.cos() * 0.0 * 3.0 / 2.0, 0.0].into(),
        );

        let matrices = vec![matrix];

        matrices
    }
}
