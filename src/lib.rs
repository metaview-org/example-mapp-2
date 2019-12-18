use std::time::Duration;
use std::io::Read;
use std::collections::VecDeque;
use ammolite_math::*;
use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use mlib::*;

macro_rules! print {
    ($mapp:ident, $($tt:tt)*) => {
        let formatted = format!($($tt)*);
        $mapp.io.out.extend(formatted.as_bytes());
    }
}

macro_rules! println {
    ($mapp:ident, $($tt:tt)*) => {
        print!($mapp, $($tt)*);
        $mapp.io.out.push('\n' as u8)
    }
}

macro_rules! eprint {
    ($mapp:ident, $($tt:tt)*) => {
        let formatted = format!($($tt)*);
        $mapp.io.err.extend(formatted.as_bytes());
    }
}

macro_rules! eprintln {
    ($mapp:ident, $($tt:tt)*) => {
        eprint!($mapp, $($tt)*);
        $mapp.io.err.push('\n' as u8)
    }
}

// Implementation from https://doc.rust-lang.org/std/macro.dbg.html
macro_rules! dbg {
    ($mapp:ident, ) => {
        eprintln!($mapp, "[{}:{}]", file!(), line!());
    };
    ($mapp:ident, $val:expr) => {
        match $val {
            tmp => {
                eprintln!($mapp, "[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($mapp:ident, $val:expr,) => { dbg!($mapp, $val) };
    ($mapp:ident, $($val:expr),+ $(,)?) => {
        ($(dbg!($mapp, $val)),+,)
    };
}

const MODEL_BYTES: &'static [u8] = include_bytes!("../../ammolite/resources/DamagedHelmet/glTF-Binary/DamagedHelmet.glb");

#[mapp]
pub struct ExampleMapp {
    io: IO,
    state: Vec<String>,
    command_id_next: usize,
    commands: VecDeque<Command>,
    root_entity: Option<Entity>,
    model: Option<Model>,
    entity: Option<Entity>,
}

impl ExampleMapp {
    fn cmd(&mut self, kind: CommandKind) {
        self.commands.push_back(Command {
            id: self.command_id_next,
            kind,
        });
        self.command_id_next += 1;
    }
}

impl Mapp for ExampleMapp {
    fn new() -> Self {
        let mut result = Self {
            io: Default::default(),
            state: Vec::new(),
            command_id_next: 0,
            commands: VecDeque::new(),
            root_entity: None,
            model: None,
            entity: None,
        };
        result.cmd(CommandKind::EntityRootGet);
        result.cmd(CommandKind::ModelCreate {
            data: (&MODEL_BYTES[..]).into(),
        });
        result.cmd(CommandKind::EntityCreate);
        result
    }

    fn test(&mut self, arg: String) -> Vec<String> {
        self.state.push(arg);
        self.state.clone()
    }

    fn update(&mut self, elapsed: Duration) {
        fn construct_model_matrix(scale: f32, translation: &Vec3, rotation: &Vec3) -> Mat4 {
            Mat4::translation(translation)
                * Mat4::rotation_roll(rotation[2])
                * Mat4::rotation_yaw(rotation[1])
                * Mat4::rotation_pitch(rotation[0])
                * Mat4::scale(scale)
        }

        if self.entity.is_none() {
            return;
        }

        let secs_elapsed = (elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000f64) as f32;
        dbg!(self, elapsed);
        dbg!(self, secs_elapsed);
        let transform = construct_model_matrix(
            1.0,
            &[0.0, 0.0, 2.0].into(),
            &[secs_elapsed.sin() * 1.0, std::f32::consts::PI + secs_elapsed.cos() * 3.0 / 2.0, 0.0].into(),
        );

        self.cmd(CommandKind::EntityTransformSet {
            entity: self.entity.unwrap(),
            transform: Some(transform),
        })
    }

    fn send_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn receive_command_response(&mut self, response: CommandResponse) {
        println!(self, "RECEIVED COMMAND RESPONSE: {:#?}", response);
        match response.kind {
            CommandResponseKind::EntityRootGet { root_entity } => {
                self.root_entity = Some(root_entity);
            },
            CommandResponseKind::ModelCreate { model } => {
                self.model = Some(model);
            }
            CommandResponseKind::EntityCreate { entity } => {
                self.entity = Some(entity);
                self.cmd(CommandKind::EntityParentSet {
                    entity: entity,
                    parent_entity: self.root_entity,
                });
                self.cmd(CommandKind::EntityModelSet {
                    entity: entity,
                    model: self.model,
                });
                self.cmd(CommandKind::EntityTransformSet {
                    entity: entity,
                    transform: Some(Mat4::identity()),
                });
            }
            _ => (),
        }
    }

    fn flush_io(&mut self) -> IO {
        std::mem::replace(&mut self.io, Default::default())
    }

    // fn get_model_matrices(&mut self, secs_elapsed: f32) -> Vec<Mat4> {
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

    //     matrices
    // }
}
