use std::time::{Duration, Instant};

use bevy::{app::AppExit, asset::LoadState, log::LogPlugin, prelude::*, winit::WinitConfig};
use crossbeam_channel::Sender;

const RUN_COUNT: usize = 10;
const ENTITY_COUNT: usize = 10_000;
const SYSTEM_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppState {
    Loading,
    Running,
    End,
}

fn main() {
    bench_with(|| path_requesting_system.system(), true);
    bench_with(|| handle_requesting_system.system(), false);
}

fn bench_with<S: System<In = (), Out = ()>>(f: impl Fn() -> S + Clone, enable_log_plugin: bool) {
    let (tx, rx) = crossbeam_channel::bounded::<Duration>(RUN_COUNT);
    for i in 0..RUN_COUNT {
        let mut app = App::build();
        app.add_plugins_with(DefaultPlugins, |builder| {
            if i != 0 || !enable_log_plugin {
                builder.disable::<LogPlugin>()
            } else {
                builder
            }
        })
        .insert_resource(tx.clone())
        .insert_resource(WinitConfig {
            return_from_run: true,
        })
        .add_state(AppState::Loading);
        app.add_system_set(
            SystemSet::on_enter(AppState::Loading).with_system(
                (|server: Res<AssetServer>, mut commands: Commands| {
                    commands.insert_resource(CommonHandle(
                        server.load::<Texture, _>("textures/array_texture.png"),
                    ))
                })
                .system(),
            ),
        );
        app.add_system_set(
            SystemSet::on_update(AppState::Loading).with_system(
                (|server: Res<AssetServer>,
                  handle: Res<CommonHandle>,
                  mut state: ResMut<State<AppState>>| {
                    if let LoadState::Loaded = server.get_load_state(handle.0.id) {
                        info!("entering AppState::Running!");
                        state.set(AppState::Running).unwrap();
                    }
                })
                .system(),
            ),
        );

        struct Start(Instant);
        app.add_system_set(
            SystemSet::on_enter(AppState::Running).with_system(
                (|mut commands: Commands| {
                    commands.insert_resource(Start(std::time::Instant::now()))
                })
                .system(),
            ),
        );

        let mut update_set = SystemSet::on_update(AppState::Running);
        for _ in 0..SYSTEM_COUNT {
            update_set = update_set.with_system(f().label("adder"));
        }
        app.add_system_set(
            update_set.with_system(
                (|mut state: ResMut<State<AppState>>| state.set(AppState::End).unwrap())
                    .system()
                    .after("adder"),
            ),
        );

        app.add_system_set(
            SystemSet::on_exit(AppState::Running).with_system(
                (|start: Res<Start>, tx: Res<Sender<Duration>>, mut exit: EventWriter<AppExit>| {
                    tx.send(std::time::Instant::now() - start.0).unwrap();
                    info!("exiting!");
                    exit.send(AppExit);
                })
                .system(),
            ),
        );
        info!("starting a run!");
        app.run();
    }
    let total: Duration = rx.try_iter().sum();
    let avg = total / RUN_COUNT as u32;
    println!("avg = {:?}", avg);
}

fn path_requesting_system(server: Res<AssetServer>, mut commands: Commands) {
    for _ in 0..ENTITY_COUNT / SYSTEM_COUNT {
        commands.spawn_bundle((server.load::<Texture, _>("textures/array_texture.png"),));
    }
}

struct CommonHandle(Handle<Texture>);
fn handle_requesting_system(handle: Res<CommonHandle>, mut commands: Commands) {
    for _ in 0..ENTITY_COUNT / SYSTEM_COUNT {
        commands.spawn_bundle((handle.0.clone(),));
    }
}
