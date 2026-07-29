use std::sync::Mutex;

use crate::{App, Target, Task};

/// Global App registry for the macro-based API.
///
/// Not meant for direct use — the [`target!`](crate::target),
/// [`task!`](crate::task), and [`run!`](crate::run) macros call
/// these functions internally.
static GLOBAL_APP: Mutex<Option<App>> = Mutex::new(None);

/// Register a target in the global app.
pub fn register_target(t: Target) {
    let mut guard = GLOBAL_APP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(App::new());
    }
    guard.as_mut().unwrap().target(t);
}

/// Register a task in the global app.
pub fn register_task(t: Task) {
    let mut guard = GLOBAL_APP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(App::new());
    }
    guard.as_mut().unwrap().task(t);
}

/// Take the global app, consuming it for use with [`run_cli`](crate::run_cli).
pub fn take_app() -> App {
    GLOBAL_APP.lock().unwrap().take().unwrap_or_default()
}
