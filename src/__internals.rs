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
        *guard = Some(App::load());
    }
    guard.as_mut().unwrap().target(t);
}

/// Register a task in the global app.
pub fn register_task(t: Task) {
    let mut guard = GLOBAL_APP.lock().unwrap();
    if guard.is_none() {
        *guard = Some(App::load());
    }
    guard.as_mut().unwrap().task(t);
}

/// Take the global app, consuming it for use with [`run_cli`](crate::run_cli).
pub fn take_app() -> App {
    GLOBAL_APP.lock().unwrap().take().unwrap_or_else(App::load)
}

// ── test helpers ────────────────────────────────────────────────────────

/// Guard that serialises macro-API test setup across parallel test threads.
///
/// Intended for integration tests that use the `target!`/`task!` macros.
/// Hold the returned guard for the entire test body to prevent other
/// test threads from interleaving their own registrations.
#[doc(hidden)]
pub struct TestSetup {
    _guard: std::sync::MutexGuard<'static, ()>,
}

#[doc(hidden)]
impl Default for TestSetup {
    fn default() -> Self {
        Self::new()
    }
}

impl TestSetup {
    pub fn new() -> Self {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        match GLOBAL_APP.lock() {
            Ok(mut app) => *app = Some(App::new()),
            Err(poisoned) => *poisoned.into_inner() = Some(App::new()),
        }
        TestSetup { _guard: guard }
    }
}
