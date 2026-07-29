use std::sync::Arc;

pub type CheckFn = Arc<dyn Fn() -> anyhow::Result<bool> + Send + Sync>;

/// A declaration of a concrete desired state.
///
/// There are two kinds:
/// - **Leaf** — has a `check` function that probes the real system.
/// - **Aggregate** — no check; its satisfaction is derived from its
///   [`depends_on`](Target::depends_on) targets.
///
/// Each target must be satisfied by **exactly one** [`Task`](crate::Task).
#[derive(Clone)]
pub struct Target {
    pub name: String,
    pub check: Option<CheckFn>,
    pub depends_on: Vec<String>,
}

impl Target {
    /// Create a new target with the given unique name.
    pub fn new(name: impl Into<String>) -> Self {
        Target {
            name: name.into(),
            check: None,
            depends_on: Vec::new(),
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

    /// Declare a dependency on another target (by name).
    ///
    /// An aggregate target has no check — it is satisfied when *all* its
    /// dependencies are satisfied. A leaf target may also declare
    /// dependencies for ordering purposes.
    pub fn depends_on(mut self, target: impl Into<String>) -> Self {
        self.depends_on.push(target.into());
        self
    }
}
