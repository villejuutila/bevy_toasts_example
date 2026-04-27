use std::{
    sync::mpsc::{self, Sender},
    time::Duration,
};

use bevy::{
    feathers::{
        FeathersPlugins,
        controls::{
            ButtonProps, ButtonVariant, SliderProps, button, menu_divider, slider, toggle_switch,
        },
        dark_theme::create_dark_theme,
        display::label,
        rounded_corners::RoundedCorners,
        theme::{ThemedText, UiTheme},
    },
    log::{
        BoxedLayer, LogPlugin,
        tracing_subscriber::{Layer, layer::Context},
    },
    platform::cell::SyncCell,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{
        Activate, SliderPrecision, SliderValue, ValueChange, checkbox_self_update,
        slider_self_update,
    },
};
use bevy_toasts_example::{ToastPosition, ToastProps, ToastVariant, ToastsPlugin, toast};
use tracing::{Level, Subscriber, field::Visit};

struct CaptureLayerVisitor<'a>(&'a mut Option<String>);

impl Visit for CaptureLayerVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn core::fmt::Debug) {
        if field.name() == "message" {
            *self.0 = Some(format!("{value:?}"));
        }
    }
}

#[derive(Resource)]
struct DesiredToastDuration(Option<f32>);

#[derive(Resource, Default)]
struct DesiredToastPosition(ToastPosition);

#[derive(Resource)]
struct LogLevelFilter(Level);

const LOG_LEVELS: [Level; 3] = [Level::INFO, Level::WARN, Level::ERROR];

impl std::fmt::Display for LogLevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Resource)]
struct LogMessageReceiver(SyncCell<mpsc::Receiver<LogMessage>>);

struct LogMessage {
    level: Level,
    message: String,
}

#[derive(Component, Default, Clone)]
struct ToastDurationSlider;

struct ConsoleLogLayer(Sender<LogMessage>);

impl<S: Subscriber> Layer<S> for ConsoleLogLayer {
    fn on_event(&self, event: &tracing::Event, _ctx: Context<'_, S>) {
        let mut message = None;

        event.record(&mut CaptureLayerVisitor(&mut message));
        let Some(message) = message else {
            return;
        };

        self.0
            .send(LogMessage {
                message,
                level: *event.metadata().level(),
            })
            .expect("Failed to send LogMessage");
    }
}

fn log_layer(app: &mut App) -> Option<BoxedLayer> {
    let (tx, rx) = mpsc::channel();
    let layer = ConsoleLogLayer(tx);
    let resource = LogMessageReceiver(SyncCell::new(rx));

    app.insert_non_send(resource);
    app.add_systems(Update, spawn_toast_on_log_message);

    Some(layer.boxed())
}

fn update_button_group_self_state<T>(
    trigger: On<ButtonGroupButtonActivated<T>>,
    mut button_variant: Query<(Entity, &mut ButtonVariant)>,
    children: Query<&Children>,
) where
    T: ToString + PartialEq + Clone + Send + Sync + 'static,
{
    for child in children.iter_descendants(trigger.entity) {
        if let Ok((entity, _)) = button_variant.get(child) {
            if let Ok((_, mut variant)) = button_variant.get_mut(entity) {
                *variant = if entity == trigger.button {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Normal
                }
            }
        }
    }
}

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            custom_layer: log_layer,
            ..Default::default()
        }))
        .add_plugins(FeathersPlugins)
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(LogLevelFilter(Level::INFO))
        .insert_resource(DesiredToastDuration(Some(5.0)))
        .init_resource::<DesiredToastPosition>()
        .add_plugins(ToastsPlugin)
        .add_systems(Startup, setup)
        .run()
}

fn setup(
    mut commands: Commands,
    level_filter: Res<LogLevelFilter>,
    toast_position: Res<DesiredToastPosition>,
) {
    commands.spawn_scene_list(bsn_list! [(
        Camera2d
    ), (
        Node {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: px(10)
        }
        Children[(
            Text("Toast position") ThemedText
        ), (
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: px(10),
            }
            Children[(
                button_group(ToastPosition::variants(), Some(&toast_position.0))
                on(update_button_group_self_state::<ToastPosition>)
                on(|trigger: On<ButtonGroupButtonActivated<ToastPosition>>, mut desired_toast_position: ResMut<DesiredToastPosition>| {
                    desired_toast_position.0 = trigger.value;
                })
            )]
        ), (
            Text("Toast duration") ThemedText
        ), (
            Node {
                flex_direction: FlexDirection::Row,
                align_content: AlignContent::Center,
                column_gap: px(10),
            }
            Children[(
                label("Indefinite")
            ), (
                toggle_switch() on(checkbox_self_update)
                on(|trigger: On<ValueChange<bool>>, mut commands: Commands, mut desired_toast_duration: ResMut<DesiredToastDuration>, duration_slider: Single<(Entity, &SliderValue), With<ToastDurationSlider>>| {
                    let (slider, value) = *duration_slider;
                    if trigger.value {
                        commands.entity(slider).insert(InteractionDisabled);
                        desired_toast_duration.0 = None;
                    } else {
                        commands.entity(slider).remove::<InteractionDisabled>();
                        desired_toast_duration.0 = Some(value.0);
                    }
                })
            )]
        ), (
            slider(SliderProps { value: 5.0, min: 0.5, max: 20.0 })
            SliderPrecision(2)
            ToastDurationSlider
            on(|trigger: On<ValueChange<f32>>, mut duration: ResMut<DesiredToastDuration>| {
                duration.0 = Some(trigger.value);
            })
            on(slider_self_update)
        ), (
            Text("Log level filter") ThemedText
        ), (
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: px(10),
            }
            Children[(
                button_group(LOG_LEVELS, Some(&level_filter.0))
                on(update_button_group_self_state::<Level>)
                on(|trigger: On<ButtonGroupButtonActivated<Level>>, mut log_level_filter: ResMut<LogLevelFilter>,| {
                    log_level_filter.0 = trigger.value;
                })
            )]
        ),
        menu_divider(),
        (
            button(ButtonProps {
                caption: Box::new(bsn_list! [ Text("Spawn Success toast") ThemedText ]),
                ..default()
            })
            on(|_: On<Activate>, mut commands: Commands, duration: Res<DesiredToastDuration>, position: Res<DesiredToastPosition>| {
                commands.spawn_scene(toast(ToastProps {
                    message: "This is a ToastVariant::Success that overflows".into(),
                    variant: ToastVariant::Success,
                    duration: duration.0.map(|dur| Duration::from_secs_f32(dur)),
                    position: Some(position.0.clone()),
                }));
            })
        ), (
            button(ButtonProps {
                caption: Box::new(bsn_list! [ Text("Write INFO log") ThemedText ]),
                ..default()
            })
            on(|_: On<Activate>| {
                info!("This is an info message!");
            })
        ), (
            button(ButtonProps { caption: Box::new(bsn_list! [ Text("Write WARNING") ThemedText ]), ..default()})
            on(|_: On<Activate>| {
                warn!("This is a warning message!");
            })
        ), (
            button(ButtonProps {
                caption: Box::new(bsn_list! [ Text("Write ERROR log") ThemedText ]),
                ..default()
            })
            on(|_: On<Activate>| {
                error!("This is an error message!");
            })
        )]
    )]);
}

fn button_group<T>(values: impl IntoIterator<Item = T>, selected: Option<&T>) -> impl Scene
where
    T: ToString + PartialEq + Clone + Send + Sync + 'static,
{
    let mut peekable = values.into_iter().enumerate().peekable();
    let mut children = Vec::new();
    while let Some((idx, value)) = peekable.next() {
        let is_first = idx == 0;
        let is_last = peekable.peek().is_none();
        let variant = if Some(&value) == selected {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        };
        let props = ButtonProps {
            variant,
            caption: Box::new(bsn_list! [ Text({value.to_string()}) ThemedText]),
            corners: if is_first && is_last {
                RoundedCorners::All
            } else if is_first {
                RoundedCorners::Left
            } else if is_last {
                RoundedCorners::Right
            } else {
                RoundedCorners::None
            },
            ..default()
        };
        children.push(bsn! {
            button(props)
            on(move |trigger: On<Activate>, mut commands: Commands, child_of: Query<&ChildOf>| {
                if let Ok(parent) = child_of.get(trigger.entity) {
                    commands.trigger(ButtonGroupButtonActivated {
                        entity: parent.0,
                        button: trigger.entity,
                        value: value.clone(),
                    });
                }
            })
        });
    }
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            column_gap: px(1)
        }
        Children [{children}]
    }
}

#[derive(EntityEvent, Debug)]
struct ButtonGroupButtonActivated<T>
where
    T: ToString + PartialEq + Clone + Send,
{
    entity: Entity,
    button: Entity,
    value: T,
}

fn spawn_toast_on_log_message(
    mut commands: Commands,
    mut receiver: NonSendMut<LogMessageReceiver>,
    level_filter: Res<LogLevelFilter>,
    duration: Res<DesiredToastDuration>,
    position: Res<DesiredToastPosition>,
) {
    for msg in receiver.0.get().try_iter() {
        if level_filter.0 > msg.level {
            continue;
        }
        let duration = duration.0.map(|dur| Duration::from_secs(dur as u64));
        let position = Some(position.0.clone());
        match msg.level {
            Level::ERROR => {
                commands.spawn_scene(toast(ToastProps {
                    message: msg.message,
                    variant: ToastVariant::Error,
                    duration,
                    position,
                }));
            }
            Level::WARN => {
                commands.spawn_scene(toast(ToastProps {
                    message: msg.message,
                    variant: ToastVariant::Warning,
                    duration,
                    position,
                }));
            }
            Level::INFO => {
                commands.spawn_scene(toast(ToastProps {
                    message: msg.message,
                    variant: ToastVariant::Info,
                    duration,
                    position,
                }));
            }
            _ => {}
        }
    }
}
