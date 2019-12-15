use ammolite_math::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use json5::{from_str, to_string};

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

// use std::marker::PhantomData;
// struct ToImpl;
// trait TheTrait {}
// struct TraitGuard<T: TheTrait>(PhantomData<T>);
// const X: TraitGuard<ToImpl> = TraitGuard(PhantomData);
// // impl TheTrait for ToImpl {}

// TO GENERATE USING A PROC MACRO:
// trait Mapp {
//     fn new() -> Self;
//     fn test(&mut self, arg: String) -> Vec<String>;
//     fn get_model_matrices(&mut self, secs_elapsed: f32) -> Vec<Mat4>;
// }

// lazy_static! {
//     static ref CTX: RwLock<Option<ExampleMapp>> = RwLock::new(None);
// }

// #[wasm_bindgen]
// pub fn initialize() {
//     *CTX.write().unwrap() = Some(<ExampleMapp as Mapp>::new());
// }

// #[wasm_bindgen]
// pub fn test(args: String) -> String {
//     let mut ctx = CTX.write().unwrap();
//     let ctx = ctx.as_mut().unwrap();
//     let (arg,) = from_str::<(String,)>(&args).unwrap();
//     let result = ctx.test(arg,);
//     to_string(&result).unwrap()
// }

// #[wasm_bindgen]
// pub fn get_model_matrices(args: String) -> String {
//     let mut ctx = CTX.write().unwrap();
//     let ctx = ctx.as_mut().unwrap();
//     let (secs_elapsed,) = from_str::<(f32,)>(&args).unwrap();
//     let result = ctx.get_model_matrices(secs_elapsed,);
//     to_string(&result).unwrap()
// }

// #[wasm_bindgen]
// pub fn test(arg: String) -> String {
//     let mut ctx = CTX.write().unwrap();
//     let ctx = ctx.as_mut().unwrap();
//     ctx.state.push(arg);
//     // let flat: Vec<f32> = Mat4::identity()
//     //     .as_slice_mut()
//     //     .into_iter()
//     //     .map(|x| *x)
//     //     .collect::<Vec<_>>();

//     // flat.into_boxed_slice()
//     to_string(&ctx.state).unwrap()
// }

// #[wasm_bindgen]
// pub fn get_model_matrices(args: String) -> String {
//     let secs_elapsed = from_str::<f32>(&args).unwrap();
//     fn construct_model_matrix(scale: f32, translation: &Vec3, rotation: &Vec3) -> Mat4 {
//         Mat4::translation(translation)
//             * Mat4::rotation_roll(rotation[2])
//             * Mat4::rotation_yaw(rotation[1])
//             * Mat4::rotation_pitch(rotation[0])
//             * Mat4::scale(scale)
//     }

//     let matrix = construct_model_matrix(
//         1.0,
//         &[0.0, 0.0, 2.0].into(),
//         &[secs_elapsed.sin() * 0.0 * 1.0, std::f32::consts::PI + secs_elapsed.cos() * 0.0 * 3.0 / 2.0, 0.0].into(),
//     );

//     let matrices = vec![matrix];

//     to_string(&matrices).unwrap()
// }
