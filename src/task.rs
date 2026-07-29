use std::sync::Arc;

pub type RunFn = Arc<dyn Fn() -> anyhow::Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct Task {
    pub name: String,
    pub satisfies: Vec<String>,
    pub depends_on: Vec<String>,
    pub labels: Vec<String>,
    pub run: Option<RunFn>,
}

impl Task {
    pub fn new(name: impl Into<String>) -> Self {
        Task {
            name: name.into(),
            satisfies: Vec::new(),
            depends_on: Vec::new(),
            labels: Vec::new(),
            run: None,
        }
    }

    pub fn satisfies(mut self, target: impl Into<String>) -> Self {
        self.satisfies.push(target.into());
        self
    }

    pub fn depends_on(mut self, target: impl Into<String>) -> Self {
        self.depends_on.push(target.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    pub fn run<F>(mut self, f: F) -> Self
    where
        F: Fn() -> anyhow::Result<()> + Send + Sync + 'static,
    {
        self.run = Some(Arc::new(f));
        self
    }
}
