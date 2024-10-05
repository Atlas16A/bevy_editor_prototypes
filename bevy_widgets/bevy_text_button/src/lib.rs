//! A simple text button widget for Bevy.
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
};

#[derive(Component)]
#[component(on_add = text_button_setup)]
pub struct WidgetTextButton(pub String);

fn text_button_setup(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let text = world.get::<WidgetTextButton>(entity).unwrap().0.clone();

    world.commands().entity(entity).with_children(|parent| {
        parent
            .spawn(NodeBundle {
                style: Style {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    height: Val::Px(30.0),
                    width: Val::Px(100.0),

                    ..Default::default()
                },
                background_color: BackgroundColor(Color::linear_rgb(0.2, 0.2, 0.2)),
                border_radius: BorderRadius {
                    top_left: Val::Px(5.0),
                    top_right: Val::Px(5.0),
                    bottom_left: Val::Px(5.0),
                    bottom_right: Val::Px(5.0),
                },
                ..Default::default()
            })
            .with_child(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: text,
                        style: TextStyle {
                            font_size: 20.0,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
                ..Default::default()
            });
    });
    world.commands().entity(entity).remove::<WidgetTextButton>();
}

/* */
