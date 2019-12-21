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

const MODEL_MAIN_BYTES: &'static [u8] = include_bytes!("../../ammolite/resources/DamagedHelmet/glTF-Binary/DamagedHelmet.glb");
const MODEL_MARKER_BYTES: &'static [u8] = include_bytes!("../../ammolite/resources/sphere_1m_radius.glb");

#[derive(Debug)]
pub struct Orientation {
    direction: Vec3,
    position: Vec3,
}

pub struct RayTracingTask {
    direction: Vec3,
    total_distance: f32,
}

#[mapp]
pub struct ExampleMapp {
    io: IO,
    state: Vec<String>,
    command_id_next: usize,
    commands: VecDeque<Command>,
    view_orientations: Option<Vec<Option<Orientation>>>,
    root_entity: Option<Entity>,
    model_main: Option<Model>,
    model_marker: Option<Model>,
    entity_main: Option<Entity>,
    entity_marker: Option<Entity>,
    ray_tracing_task: Option<RayTracingTask>,
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
            view_orientations: None,
            root_entity: None,
            model_main: None,
            model_marker: None,
            entity_main: None,
            entity_marker: None,
            ray_tracing_task: None,
        };
        result.cmd(CommandKind::EntityRootGet);
        result.cmd(CommandKind::ModelCreate {
            data: (&MODEL_MAIN_BYTES[..]).into(),
        });
        result.cmd(CommandKind::ModelCreate {
            data: (&MODEL_MARKER_BYTES[..]).into(),
        });
        result.cmd(CommandKind::EntityCreate);
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

        if self.entity_main.is_none() {
            return;
        }

        let secs_elapsed = (elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000f64) as f32;
        // dbg!(self, elapsed);
        // dbg!(self, secs_elapsed);
        let transform = construct_model_matrix(
            1.0,
            &[0.0, 0.0, 2.0].into(),
            &[secs_elapsed.sin() * 1.0, std::f32::consts::PI + secs_elapsed.cos() * 3.0 / 2.0, 0.0].into(),
        );

        self.cmd(CommandKind::EntityTransformSet {
            entity: self.entity_main.unwrap(),
            transform: Some(transform),
        });
        self.cmd(CommandKind::GetViewOrientation {});
    }

    fn send_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn receive_command_response(&mut self, response: CommandResponse) {
        // println!(self, "RECEIVED COMMAND RESPONSE: {:#?}", response);
        match response.kind {
            CommandResponseKind::EntityRootGet { root_entity } => {
                self.root_entity = Some(root_entity);
            },
            CommandResponseKind::ModelCreate { model } => {
                if self.model_main.is_none() {
                    self.model_main = Some(model);
                } else {
                    self.model_marker = Some(model);
                }
            },
            CommandResponseKind::EntityCreate { entity } => {
                let model_selector = {
                    let (entity_selector, model_selector) = if self.entity_main.is_none() {
                        (&mut self.entity_main, self.model_main)
                    } else {
                        (&mut self.entity_marker, self.model_marker)
                    };
                    *entity_selector = Some(entity);
                    model_selector
                };
                self.cmd(CommandKind::EntityParentSet {
                    entity: entity,
                    parent_entity: self.root_entity,
                });
                self.cmd(CommandKind::EntityModelSet {
                    entity: entity,
                    model: model_selector,
                });
                self.cmd(CommandKind::EntityTransformSet {
                    entity: entity,
                    transform: Some(Mat4::identity()),
                });
            },
            CommandResponseKind::GetViewOrientation { views_per_medium } => {
                // dbg!(self, &views_per_medium);

                self.view_orientations = Some(views_per_medium.into_iter()
                    .map(|views|
                        views.map(|views| {
                            let views_len = views.len();
                            let mut average_view = Mat4::zero();

                            for view in views {
                                average_view = average_view + view.pose;
                            }

                            average_view = average_view / (views_len as f32);
                            average_view
                        })
                        .map(|average_view| {
                            Orientation {
                                // Investigate why -z is needed instead of +z
                                direction: (&average_view * Vec4([0.0, 0.0, -1.0, 0.0])).into_projected(),
                                position:  (&average_view * Vec4([0.0, 0.0, 0.0, 1.0])).into_projected(),
                            }
                        })
                    ).collect::<Vec<_>>());

                // dbg!(self, &self.view_orientations);

                let ray_trace_cmd = self.view_orientations.as_ref().and_then(|view_orientations| {
                    if let [Some(hmd), _] = &view_orientations[..] {
                        Some((hmd.position.clone(), hmd.direction.clone()))
                    } else {
                        None
                    }
                });

                if let Some((position, direction)) = ray_trace_cmd {
                    self.ray_tracing_task = Some(RayTracingTask {
                        direction: direction.clone(),
                        total_distance: 0.0,
                    });
                    self.cmd(CommandKind::RayTrace {
                        origin: position,
                        direction: direction,
                    });
                }
            },
            CommandResponseKind::RayTrace { closest_intersection } => {
                // dbg!(self, &closest_intersection);

                if let Some(closest_intersection) = closest_intersection {
                    let RayTracingTask {
                        direction,
                        total_distance,
                    } = self.ray_tracing_task.take().unwrap();
                    let previous_total_distance = total_distance;
                    let total_distance = previous_total_distance + closest_intersection.distance_from_origin;

                    // Continue ray tracing from current intersection, if marker hit
                    if Some(closest_intersection.entity) == self.entity_marker {
                        self.cmd(CommandKind::RayTrace {
                            origin: closest_intersection.position + (&direction * (8.0 * std::f32::EPSILON)),
                            direction: direction.clone(),
                        });

                        self.ray_tracing_task = Some(RayTracingTask {
                            direction: direction,
                            total_distance,
                        });
                    } else {
                        let scale = 0.02 * total_distance;
                        let transform = Mat4::translation(&closest_intersection.position)
                            * Mat4::scale(scale);

                        self.cmd(CommandKind::EntityModelSet {
                            entity: self.entity_marker.unwrap(),
                            model: self.model_marker,
                        });
                        self.cmd(CommandKind::EntityTransformSet {
                            entity: self.entity_marker.unwrap(),
                            transform: Some(transform),
                        });

                        self.ray_tracing_task = None;
                    }
                } else {
                    self.cmd(CommandKind::EntityModelSet {
                        entity: self.entity_marker.unwrap(),
                        model: None,
                    });
                }
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
