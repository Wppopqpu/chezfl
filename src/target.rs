use std::sync::Arc;

pub type CheckFn = Arc<dyn Fn() -> anyhow::Result<bool> + Send + Sync>;

/// A declaration of a concrete desired state.
///
/// Three kinds:
/// - **Leaf** — has a `check` function, optionally guarded by
///   [`check_deps`](Target::check_dep).
/// - **Aggregate** — no check; satisfaction derived from its
///   [`depends_on`](Target::depends_on) targets.
/// - **Stub** — neither check nor depends_on.
///
/// Each target must be satisfied by **exactly one** [`Task`](crate::Task).
#[derive(Clone)]
pub struct Target {
    pub name: String,
    pub description: Option<String>,
    pub check: Option<CheckFn>,
    pub depends_on: Vec<String>,
    pub check_deps: Vec<String>,
}

impl Target {
    /// Create a new target with the given unique name.
    pub fn new(name: impl Into<String>) -> Self {
        Target {
            name: name.into(),
            description: None,
            check: None,
            depends_on: Vec::new(),
            check_deps: Vec::new(),
        }
    }

    /// Set the check function for a leaf target.
    ///
    /// The closure returns `Ok(true)` when the state is satisfied,
    /// `Ok(false)` when unsatisfied, or `Err` if the check itself failed.
    pub fn check<F>(mut self, f: F) -> Self
    where
        F: Fn() -> anyhow::Result<bool> + Send + Sync + 'static,
    {
        self.check = Some(Arc::new(f));
        self
    }

    /// Attach a human-readable description of this target.
    ///
    /// Shown always when the target is unsatisfied, and optionally in
    /// dim text when `--show-descriptions` is passed.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Declare a dependency on another target (by name).
    ///
    /// An aggregate target has no check — it is satisfied when *all* its
    /// dependencies are satisfied. A leaf target may also declare
    /// dependencies for ordering purposes.
    pub fn depends_on(mut self, target: impl Into<String>) -> Self {
        self.depends_on.push(target.into());
        self
    }

    /// Declare a check dependency (by name).
    ///
    /// A leaf target's `check` only runs when **all** its check deps are
    /// satisfied. If any check dep is unsatisfied the target is demoted
    /// to stub — the check is skipped and no task runs for it.
    pub fn check_dep(mut self, target: impl Into<String>) -> Self {
        self.check_deps.push(target.into());
        self
    }

    /// A stub target has neither a `check` function nor `depends_on`.
    /// It is always unsatisfied unless manually set via `--set`.
    pub fn is_stub(&self) -> bool {
        self.check.is_none() && self.depends_on.is_empty()
    }
}
