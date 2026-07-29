use std::sync::Arc;

pub type CheckFn = Arc<dyn Fn() -> anyhow::Result<bool>>;

#[derive(Clone)]
pub struct Target {
    pub name: String,
    pub check: Option<CheckFn>,
    pub depends_on: Vec<String>,
}

impl Target {
    pub fn new(name: impl Into<String>) -> Self {
        Target {
            name: name.into(),
            check: None,
            depends_on: Vec::new(),
        }
    }

    pub fn check<F>(mut self, f: F) -> Self
    where
        F: Fn() -> anyhow::Result<bool> + 'static,
    {
        self.check = Some(Arc::new(f));
        self
    }

    pub fn depends_on(mut self, target: impl Into<String>) -> Self {
        self.depends_on.push(target.into());
        self
    }
}
