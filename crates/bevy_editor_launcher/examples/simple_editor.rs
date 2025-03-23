//! A simple example of how to launch the editor.
use bevy::prelude::*;

#[cfg(not(feature = "editor"))]
fn main() {
    App::new().add_plugins(DefaultPlugins).run();
}

#[cfg(feature = "editor")]
use bevy_editor::EditorPlugin;
#[cfg(feature = "editor")]
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EditorPlugin)
        .run();
}
