use std::sync::Arc;

pub type RunFn = Arc<dyn Fn() -> anyhow::Result<()> + Send + Sync>;

/// An actionable unit that satisfies one or more [targets](crate::Target).
///
/// A task declares:
/// - which targets it [`satisfies`](Task::satisfies) (1-to-many)
/// - which other targets must be satisfied before it can
///   [`run`](Task::depends_on) (dependency on *targets*, not other tasks)
/// - [`labels`](Task::label) for filtering and disabling
///
/// Tasks are **idempotent** — designed to be run repeatedly. Execution is
/// **serial** to support interactive programs. There is no rollback.
#[derive(Clone)]
pub struct Task {
    pub name: String,
    pub description: Option<String>,
    pub satisfies: Vec<String>,
    pub depends_on: Vec<String>,
    pub labels: Vec<String>,
    pub run: Option<RunFn>,
}

impl Task {
    /// Create a new task with the given unique name.
    pub fn new(name: impl Into<String>) -> Self {
        Task {
            name: name.into(),
            description: None,
            satisfies: Vec::new(),
            depends_on: Vec::new(),
            labels: Vec::new(),
            run: None,
        }
    }

    /// Attach a human-readable description of what this task does.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Declare that this task satisfies the named target.
    ///
    /// A single task can satisfy multiple targets. Each target can be
    /// satisfied by at most one task.
    pub fn satisfies(mut self, target: impl Into<String>) -> Self {
        self.satisfies.push(target.into());
        self
    }

    /// Declare a dependency on a target being satisfied before this task runs.
    ///
    /// This creates an ordering constraint: the task will not execute until
    /// the named target reports satisfied.
    pub fn depends_on(mut self, target: impl Into<String>) -> Self {
        self.depends_on.push(target.into());
        self
    }

    /// Add a label for filtering and disabling.
    ///
    /// Users can exclude tasks by label via `--exclude-label` or select
    /// tasks by label via `--label`. Disabled tasks are completely ignored
    /// and their targets are skipped.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Set the run function.
    ///
    /// The closure should be idempotent. stdout/stderr are forwarded to
    /// the terminal; stdin is passed through so interactive programs
    /// (e.g. `yay`, `sudo`) work normally.
    pub fn run<F>(mut self, f: F) -> Self
    where
        F: Fn() -> anyhow::Result<()> + Send + Sync + 'static,
    {
        self.run = Some(Arc::new(f));
        self
    }
}
