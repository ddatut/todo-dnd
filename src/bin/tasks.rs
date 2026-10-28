//! Plain to-do window. No console, and nothing on screen that looks like a game.
#![windows_subsystem = "windows"]

fn main() {
    todo_dnd::load_local_env();
    let _ = todo_dnd::gui::run_work();
}
