use std::sync::Mutex;
use std::time::Duration;
use std::collections::VecDeque;
use include_dir::{include_dir, DirEntry, FileSystem};
use regex::Regex;
use lazy_static::lazy_static;
use ammolite_math::*;
use wasm_bindgen::prelude::*;
use ::mlib::*;

#[derive(Default)]
pub struct SyncIO {
    out: Mutex<Vec<u8>>,
    err: Mutex<Vec<u8>>,
}

impl SyncIO {
    pub fn flush(&self) -> IO {
        IO {
            out: {
                let mut mut_out = GLOBAL_IO.out.lock().expect("Could not lock the stdout.");
                std::mem::replace(mut_out.as_mut(), Vec::new())
            },
            err: {
                let mut mut_err = GLOBAL_IO.err.lock().expect("Could not lock the stderr.");
                std::mem::replace(mut_err.as_mut(), Vec::new())
            },
        }
    }

    pub fn write_out<T: AsRef<[u8]>>(&self, bytes: impl IntoIterator<Item=T>) {
        let mut mut_out = GLOBAL_IO.out.lock().expect("Could not lock the stdout.");

        for item in bytes {
            mut_out.extend(item.as_ref());
        }
    }

    pub fn write_err<T: AsRef<[u8]>>(&self, bytes: impl IntoIterator<Item=T>) {
        let mut mut_err = GLOBAL_IO.err.lock().expect("Could not lock the stderr.");

        for item in bytes {
            mut_err.extend(item.as_ref());
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_IO: SyncIO = Default::default();
}

#[allow(unused)]
macro_rules! print {
    ($($tt:tt)*) => {
        let formatted = format!($($tt)*);
        GLOBAL_IO.write_out(&[formatted.as_bytes()]);
    }
}

#[allow(unused)]
macro_rules! println {
    ($($tt:tt)*) => {
        let formatted = format!($($tt)*);
        GLOBAL_IO.write_out(&[
            formatted.as_bytes(),
            std::slice::from_ref(&('\n' as u8))
        ]);
    }
}

#[allow(unused)]
macro_rules! eprint {
    ($($tt:tt)*) => {
        let formatted = format!($($tt)*);
        GLOBAL_IO.write_err(&[formatted.as_bytes()]);
    }
}

#[allow(unused)]
macro_rules! eprintln {
    ($($tt:tt)*) => {
        let formatted = format!($($tt)*);
        GLOBAL_IO.write_err(&[
            formatted.as_bytes(),
            std::slice::from_ref(&('\n' as u8))
        ]);
    }
}

// Implementation from https://doc.rust-lang.org/std/macro.dbg.html
#[allow(unused)]
macro_rules! dbg {
    () => {
        eprintln!("[{}:{}]", file!(), line!());
    };
    ($val:expr) => {
        match $val {
            tmp => {
                eprintln!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($val:expr,) => { dbg!($val) };
    ($($val:expr),+ $(,)?) => {
        ($(dbg!($val)),+,)
    };
}

const SELECTION_DELAY: f32 = 1.0;
const ANIMATION_SPEED: f32 = 0.0;

const MODEL_MARKER_BYTES: &'static [u8] = include_bytes!("../resources/ui/sphere_1m_radius.glb");

const DIR: FileSystem = include_dir!("resources/showcase");

lazy_static! {
    static ref GLOBAL_TRANSFORMATION: Mat4 = Mat4::rotation_yaw(0.0 * std::f32::consts::PI);
    static ref MODELS_MAIN_BYTES_SCALE: Vec<(&'static [u8], f32)> = {
        let files = DIR.find("**/*_(*).glb")
            .expect("Could not traverse the resource directory tree.")
            .flat_map(|dir_entry| {
                match dir_entry {
                    DirEntry::File(file) => Some(file),
                    DirEntry::Dir(_) => None,
                }
            })
            .collect::<Vec<_>>();

        if files.len() <= 0 {
            panic!("No `.glb` glTF models in the `resources/showcase` directory.")
        }

        println!("Packaged showcase models:");

        files.into_iter()
            .enumerate()
            .map(|(index, file)| {
                lazy_static! {
                    static ref PATTERN: Regex = Regex::new(r"^(?P<name>.*)_\((?P<scale>.*?)\)$").unwrap();
                }
                let stem = file.path().file_stem().unwrap().to_str().unwrap();
                let captures = PATTERN.captures(stem).unwrap();
                let scale = captures.name("scale").unwrap().as_str().parse().unwrap();

                println!("Model #{}: {:?}", index, file.path());

                (file.contents(), scale)
            })
            .collect::<Vec<_>>()
    };
}

fn construct_model_matrix(scale: f32, translation: &Vec3, rotation: &Vec3) -> Mat4 {
    Mat4::translation(translation)
        * Mat4::rotation_roll(rotation[2])
        * Mat4::rotation_yaw(rotation[1])
        * Mat4::rotation_pitch(rotation[0])
        * Mat4::scale(scale)
}

fn duration_to_seconds(duration: Duration) -> f32 {
    (duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000f64) as f32
}

#[derive(Debug, Clone)]
pub struct Orientation {
    direction: Vec3,
    position: Vec3,
}

pub struct RayTracingTask {
    transform: Mat4,
    direction: Vec3,
    total_distance: f32,
}

pub struct Grab {
    entity: Entity,
    original_transform: Mat4,
}

#[mapp]
pub struct ExampleMapp {
    elapsed: Duration,
    command_id_next: usize,
    commands: VecDeque<Command>,
    view_orientations: Option<Vec<Option<(Mat4, Orientation)>>>,
    primary_view_orientation: Option<(Mat4, Orientation)>,
    root_entity: Option<Entity>,
    models_main: Vec<Option<Model>>,
    model_marker: Option<Model>,
    entities_main: Vec<Option<Entity>>,
    transforms_main: Vec<Option<Mat4>>,
    entity_marker: Option<Entity>,
    ray_tracing_task: Option<RayTracingTask>,
    grabbed_entity: Option<Grab>,
    grab_entity: bool,
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
            elapsed: Default::default(),
            command_id_next: 0,
            commands: VecDeque::new(),
            view_orientations: None,
            primary_view_orientation: None,
            root_entity: None,
            models_main: vec![None; MODELS_MAIN_BYTES_SCALE.len()],
            model_marker: None,
            entities_main: vec![None; MODELS_MAIN_BYTES_SCALE.len()],
            transforms_main: vec![None; MODELS_MAIN_BYTES_SCALE.len()],
            entity_marker: None,
            ray_tracing_task: None,
            grabbed_entity: None,
            grab_entity: false,
        };
        result.cmd(CommandKind::EntityRootGet);

        // Load models
        for (model_bytes, _) in &MODELS_MAIN_BYTES_SCALE[..] {
            result.cmd(CommandKind::ModelCreate {
                data: model_bytes.into(),
            });
        }
        result.cmd(CommandKind::ModelCreate {
            data: (&MODEL_MARKER_BYTES[..]).into(),
        });

        // Create entities
        for _ in 0..MODELS_MAIN_BYTES_SCALE.len() {
            result.cmd(CommandKind::EntityCreate);
        }
        result.cmd(CommandKind::EntityCreate);

        result
    }

    fn update(&mut self, elapsed: Duration) {
        self.elapsed = elapsed;

        for (index, entity) in self.entities_main.clone().iter().enumerate() {
            if entity.is_none() {
                return;
            }

            let entity = entity.as_ref().unwrap();
            let mut transform = self.transforms_main[index].as_ref().unwrap().clone();

            if self.grabbed_entity.is_some() && self.grabbed_entity.as_ref().unwrap().entity == *entity {
                let Grab {
                    entity,
                    original_transform,
                } = self.grabbed_entity.as_ref().unwrap();
                let current_transform = self.primary_view_orientation.as_ref().unwrap().0.clone();
                transform = current_transform * original_transform.inverse() * transform;
            }

            self.cmd(CommandKind::EntityTransformSet {
                entity: *entity,
                transform: Some(&*GLOBAL_TRANSFORMATION * transform),
            });
        }
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
                let model_ref = if let Some(model_main) = self.models_main.iter_mut().find(|model_main| model_main.is_none()) {
                    model_main
                } else if self.model_marker.is_none() {
                    &mut self.model_marker
                } else {
                    panic!("Too many ModelCreate commands sent.");
                };

                *model_ref = Some(model);
            },
            CommandResponseKind::EntityCreate { entity } => {
                let (model_selector, transform_selector, transform) = {
                    let (entity_selector, transform_selector, model_selector, transform) = if let Some(index) = self.entities_main.iter().position(|entity| entity.is_none()) {
                        (
                            &mut self.entities_main[index],
                            Some(&mut self.transforms_main[index]),
                            self.models_main[index],
                            Mat4::translation(&[0.0, 0.0, 2.0 + 4.0 * index as f32].into())
                            * Mat4::scale(MODELS_MAIN_BYTES_SCALE[index].1)
                        )
                    } else if self.entity_marker.is_none() {
                        (&mut self.entity_marker, None, self.model_marker, Mat4::IDENTITY)
                    } else {
                        panic!("Too many EntityCreate commands sent.");
                    };
                    *entity_selector = Some(entity);
                    (model_selector, transform_selector, transform)
                };
                if let Some(transform_selector) = transform_selector {
                    *transform_selector = Some(transform.clone());
                }
                self.cmd(CommandKind::EntityParentSet {
                    entity,
                    parent_entity: self.root_entity,
                });
                self.cmd(CommandKind::EntityModelSet {
                    entity,
                    model: model_selector,
                });
                self.cmd(CommandKind::EntityTransformSet {
                    entity,
                    transform: Some(&*GLOBAL_TRANSFORMATION * transform),
                });
            },
            CommandResponseKind::GetViewOrientation { views_per_medium } => {
                self.view_orientations = Some(views_per_medium.into_iter()
                    .map(|views|
                        views.map(|views| {
                            let views_len = views.len();
                            let mut average_view = Mat4::ZERO;

                            for view in views {
                                average_view = average_view + view.pose;
                            }

                            average_view = average_view / (views_len as f32);
                            average_view
                        })
                        .map(|average_view| {
                            (
                                average_view.clone(),
                                Orientation {
                                    // Investigate why -z is needed instead of +z
                                    direction: (&average_view * Vec4([0.0, 0.0, -1.0, 0.0])).into_projected(),
                                    position:  (&average_view * Vec4([0.0, 0.0, 0.0, 1.0])).into_projected(),
                                },
                            )
                        })
                    ).collect::<Vec<_>>());

                self.primary_view_orientation = self.view_orientations.as_ref().and_then(|view_orientations| {
                    if let [Some((transform, orientation))] = &view_orientations[..] {
                        Some((transform.clone(), orientation.clone()))
                    // HMD:
                    } else if let [Some((transform, orientation)), _] = &view_orientations[..] {
                        Some((transform.clone(), orientation.clone()))
                    } else {
                        None
                    }
                });

                let ray_trace_cmd = self.view_orientations.as_ref().and_then(|view_orientations| {
                    if let [Some((transform, any))] = &view_orientations[..] {
                        Some((transform, any.position.clone(), any.direction.clone()))
                    } else if let [Some((transform, hmd)), _] = &view_orientations[..] {
                        Some((transform, hmd.position.clone(), hmd.direction.clone()))
                    } else {
                        None
                    }
                });

                if let Some((transform, position, direction)) = ray_trace_cmd {
                    self.ray_tracing_task = Some(RayTracingTask {
                        transform: transform.clone(),
                        direction: direction.clone(),
                        total_distance: 0.0,
                    });
                    self.cmd(CommandKind::RayTrace {
                        origin: position,
                        direction,
                    });
                }
            },
            CommandResponseKind::RayTrace { closest_intersection } => {
                // dbg!(self, &closest_intersection);

                if let Some(closest_intersection) = closest_intersection {
                    let RayTracingTask {
                        transform: ray_tracing_transform,
                        direction,
                        total_distance,
                    } = self.ray_tracing_task.take().unwrap();
                    let previous_total_distance = total_distance;
                    let total_distance = previous_total_distance + closest_intersection.distance_from_origin;

                    // Continue ray tracing from current intersection, if marker hit
                    if Some(closest_intersection.entity) == self.entity_marker {
                        self.cmd(CommandKind::RayTrace {
                            origin: closest_intersection.position + (&direction * (32.0 * std::f32::EPSILON)),
                            direction: direction.clone(),
                        });

                        self.ray_tracing_task = Some(RayTracingTask {
                            transform: ray_tracing_transform,
                            direction,
                            total_distance,
                        });
                    } else {
                        let transform = Mat4::translation(&closest_intersection.position)
                            * Mat4::scale(0.02 * total_distance);

                        self.cmd(CommandKind::EntityModelSet {
                            entity: self.entity_marker.unwrap(),
                            model: self.model_marker,
                        });
                        self.cmd(CommandKind::EntityTransformSet {
                            entity: self.entity_marker.unwrap(),
                            transform: Some(&*GLOBAL_TRANSFORMATION * transform),
                        });

                        self.ray_tracing_task = None;

                        if self.grab_entity && self.grabbed_entity.is_none() {
                            self.grabbed_entity = Some(Grab {
                                entity: closest_intersection.entity,
                                original_transform: ray_tracing_transform.clone(),
                            });
                        }
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

    fn receive_event(&mut self, event: Event) {
        match event {
            mlib::Event::Window(
                mlib::event::WindowEvent::MouseInput {
                    button: mlib::event::MouseButton::Left,
                    state,
                    ..
                }
            ) => {
                match state {
                    mlib::event::ElementState::Pressed => {
                        println!("Grab initiated.");
                        self.grab_entity = true;
                    },
                    mlib::event::ElementState::Released => {
                        println!("Grab ended.");
                        self.grab_entity = false;

                        if let Some(grabbed_entity) = &self.grabbed_entity {
                            let Grab {
                                entity: _,
                                original_transform,
                            } = self.grabbed_entity.as_ref().unwrap();
                            let entity_index = self.entities_main.iter().enumerate()
                                .find(|(_, entity)| entity == &&Some(grabbed_entity.entity))
                                .map(|(index, _)| index)
                                .unwrap();

                            let current_transform = self.primary_view_orientation.as_ref().unwrap().0.clone();
                            self.transforms_main[entity_index] =
                                Some(current_transform
                                    * original_transform.inverse()
                                    * self.transforms_main[entity_index].as_ref().unwrap());
                        }

                        self.grabbed_entity = None;
                    },
                }
            },
            _ => (),
        }
    }

    fn flush_io(&mut self) -> IO {
        GLOBAL_IO.flush()
    }
}
