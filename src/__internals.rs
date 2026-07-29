use std::sync::Mutex;

use crate::{App, Target, Task};

static GLOBAL_APP: Mutex<Option<App>> = Mutex::new(None);

pub fn register_target(t: Target) {
    let mut guard = GLOBAL_APP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(App::new());
    }
    guard.as_mut().unwrap().target(t);
}

pub fn register_task(t: Task) {
    let mut guard = GLOBAL_APP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(App::new());
    }
    guard.as_mut().unwrap().task(t);
}

pub fn take_app() -> App {
    GLOBAL_APP.lock().unwrap().take().unwrap_or_default()
}
