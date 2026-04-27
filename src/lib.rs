use std::time::Duration;

use accesskit::Role;
use bevy::{
    a11y::AccessibilityNode,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

pub struct ToastsPlugin;

impl Plugin for ToastsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ToastColors>();
        app.add_systems(Update, tick_toasts);
    }
}

#[derive(Resource)]
pub struct ToastColors {
    pub info: ToastColor,
    pub success: ToastColor,
    pub warning: ToastColor,
    pub error: ToastColor,
}

impl Default for ToastColors {
    fn default() -> Self {
        Self {
            info: ToastColor::new(Color::srgb(0.29, 0.65, 0.93), Color::WHITE),
            success: ToastColor::new(Color::srgb(0.4, 0.67, 0.35), Color::WHITE),
            warning: ToastColor::new(Color::srgb(0.94, 0.61, 0.21), Color::BLACK),
            error: ToastColor::new(Color::srgb(0.86, 0.36, 0.33), Color::WHITE),
        }
    }
}

impl ToastColors {
    pub fn get(&self, variant: ToastVariant) -> &ToastColor {
        match variant {
            ToastVariant::Info => &self.info,
            ToastVariant::Success => &self.success,
            ToastVariant::Warning => &self.warning,
            ToastVariant::Error => &self.error,
        }
    }
}

pub struct ToastColor {
    pub background: Color,
    pub text: Color,
}

impl ToastColor {
    pub fn new(background: Color, text: Color) -> Self {
        Self { background, text }
    }
}

#[derive(Component, Default, Reflect, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Default)]
pub enum ToastVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Component, FromTemplate)]
#[require(
    ToastVariant,
    ToastPosition,
    AccessibilityNode(accesskit::Node::new(Role::Alert))
)]
#[component(on_add)]
pub struct Toast {
    pub message: String,
    pub duration: Option<Timer>,
}

impl Toast {
    pub fn on_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        let this_position = *world
            .entity(entity)
            .get_components::<&ToastPosition>()
            .expect("Toast should have ToastPosition when added");
        let container = world
            .try_query::<(Entity, &ToastContainerPosition)>()
            .and_then(|mut q| {
                q.iter(&world)
                    .find(|(_, container_position)| **container_position == this_position)
                    .map(|(entity, _)| entity)
            });

        let parent = match container {
            Some(container) => container,
            None => world
                .commands()
                .spawn_scene(spawn_container(this_position.into()))
                .id(),
        };

        world.commands().entity(entity).set_parent_in_place(parent);
    }
}

#[derive(Component, Default, Clone, Debug, Copy)]
pub enum ToastContainerPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl PartialEq<ToastPosition> for ToastContainerPosition {
    fn eq(&self, other: &ToastPosition) -> bool {
        matches!(
            (self, other),
            (ToastContainerPosition::TopLeft, ToastPosition::TopLeft)
                | (ToastContainerPosition::TopRight, ToastPosition::TopRight)
                | (
                    ToastContainerPosition::BottomLeft,
                    ToastPosition::BottomLeft
                )
                | (
                    ToastContainerPosition::BottomRight,
                    ToastPosition::BottomRight
                )
        )
    }
}

impl From<ToastPosition> for ToastContainerPosition {
    fn from(position: ToastPosition) -> Self {
        match position {
            ToastPosition::TopLeft => ToastContainerPosition::TopLeft,
            ToastPosition::TopRight => ToastContainerPosition::TopRight,
            ToastPosition::BottomLeft => ToastContainerPosition::BottomLeft,
            ToastPosition::BottomRight => ToastContainerPosition::BottomRight,
        }
    }
}

#[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
pub enum ToastPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl ToastPosition {
    pub const fn variants() -> [ToastPosition; 4] {
        [
            ToastPosition::TopLeft,
            ToastPosition::TopRight,
            ToastPosition::BottomLeft,
            ToastPosition::BottomRight,
        ]
    }
}

impl std::fmt::Display for ToastPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToastPosition::TopLeft => write!(f, "Top Left"),
            ToastPosition::TopRight => write!(f, "Top Right"),
            ToastPosition::BottomLeft => write!(f, "Bottom Left"),
            ToastPosition::BottomRight => write!(f, "Bottom Right"),
        }
    }
}

#[derive(Component, FromTemplate)]
#[require(ToastContainerPosition)]
#[component(on_add)]
pub struct ToastContainer;

impl ToastContainer {
    pub fn on_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        let mut entity = world.entity_mut(entity);
        let (mut node, position) = entity
            .get_components_mut::<(&mut Node, &ToastContainerPosition)>()
            .expect("ToastContainer should have Node component");

        match position {
            ToastContainerPosition::TopLeft => {
                node.top = px(10);
            }
            ToastContainerPosition::TopRight => {
                node.top = px(10);
                node.right = px(10);
            }
            ToastContainerPosition::BottomLeft => {
                node.bottom = px(10);
            }
            ToastContainerPosition::BottomRight => {
                node.bottom = px(10);
                node.right = px(10);
            }
        }
    }
}

#[derive(Component, FromTemplate)]
pub struct ToastProgressBar;

fn spawn_container(position: ToastContainerPosition) -> impl Scene {
    bsn! {
        #ToastContainer
        ToastContainer
        ZIndex(1000)
        Pickable::IGNORE
        template_value(position)
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::ColumnReverse,
        }
    }
}

pub fn toast_root() -> impl Scene {
    bsn! {
        Node {
            width: px(300),
            height: px(60),
            margin: UiRect::all(px(5)),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(4))
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Relative
        }
    }
}

pub fn toast_text(text: String, variant: ToastVariant) -> impl Scene {
    bsn! {
        Node {
            width: percent(90),
            overflow: Overflow::clip_x()
        }
        Children[(
            Text({text})
            TextFont { font_size: FontSize::Px(16.0)}
            TextLayout { linebreak: LineBreak::NoWrap,  }
            template(move |ctx| {
                let text_color = ctx.resource::<ToastColors>().get(variant).text;
                Ok(TextColor(text_color))
            })
        )]

    }
}

pub fn toast_close_icon() -> impl Scene {
    bsn! {
        Button
        ImageNode { image: "close.png" }
        Node {
            width: px(30),
            height: px(30)
        }
        on(|trigger: On<Pointer<Click>>, mut commands: Commands, child_of: Query<&ChildOf>| {
            let parent = child_of.get(trigger.entity).expect("Should have ChildOf in close button on_click observer").0;
            commands.entity(parent).despawn();
        })
    }
}

#[derive(Default)]
pub struct ToastProps {
    pub message: String,
    pub variant: ToastVariant,
    pub duration: Option<Duration>,
    pub position: Option<ToastPosition>,
}

pub fn toast(props: ToastProps) -> impl Scene {
    bsn! {
        #Toast
        :toast_root
        Pickable::IGNORE
        template_value(props.variant)
        Toast {
            message: { props.message.clone() },
            duration: { props.duration.map(|dur| Timer::new(dur, TimerMode::Once)) },
        }
        template(move |_ctx| {
            Ok(props.position.unwrap_or_default())
        })
        template(move |ctx| {
            let background_color = ctx.resource::<ToastColors>().get(props.variant).background;
            Ok(BackgroundColor(background_color))
        })
        Children[
            :toast_text(props.message, props.variant),
            :toast_close_icon,
            :toast_progres_bar(props.duration.is_none())
        ]
    }
}

fn toast_progres_bar(indefinite: bool) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: px(10),
            position_type: PositionType::Absolute,
            bottom: px(0),
            left: px(0),
        }
        template(move |_| {
            let color = Color::WHITE;
            if indefinite {
                Ok(BackgroundColor(color.with_alpha(0.0)))
            } else {
                Ok(BackgroundColor(color.with_alpha(0.5)))
            }
        })
        ToastProgressBar
    }
}

fn tick_toasts(
    mut commands: Commands,
    mut toasts: Query<(Entity, &mut Toast)>,
    children: Query<&Children>,
    mut progress_bars: Query<&mut Node, With<ToastProgressBar>>,
    time: Res<Time>,
) {
    for (entity, mut toast) in toasts.iter_mut() {
        if let Some(duration) = &mut toast.duration {
            duration.tick(time.delta());
            for child in children.iter_descendants(entity) {
                if let Ok(mut progress_bar) = progress_bars.get_mut(child) {
                    let remaining_percent =
                        duration.remaining_secs() / duration.duration().as_secs() as f32;
                    progress_bar.width = percent(remaining_percent * 100.);
                }
            }
            if duration.just_finished() {
                commands.entity(entity).despawn();
            }
        }
    }
}
