use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::Satisfaction;
use crate::Step;
use crate::state::State;
use crate::target::Target;
use crate::task::Task;

/// Execution configuration for a check/apply/plan run.
#[derive(Clone, Default)]
pub struct Config {
    /// Only consider tasks that have at least one of these labels.
    /// `None` means no filter — consider all tasks.
    pub label_filter: Option<Vec<String>>,
    /// Exclude tasks that have any of these labels.
    pub exclude_labels: Vec<String>,
}

/// A registry of targets and tasks, and the execution engine.
///
/// Use [`App::new`] for a clean slate, [`App::load`] to restore from disk,
/// or [`App::with_state_path`] for a custom state file location.
pub struct App {
    targets: HashMap<String, Target>,
    tasks: HashMap<String, Task>,
    state: State,
    state_path: Option<PathBuf>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create an empty app with no state file.
    ///
    /// Targets and tasks must be added manually. State is held in memory only
    /// (no persistence).
    pub fn new() -> Self {
        App {
            targets: HashMap::new(),
            tasks: HashMap::new(),
            state: State::default(),
            state_path: None,
        }
    }

    /// Create an app restored from the default state file
    /// (`~/.local/state/chezfl/state.toml`).
    ///
    /// State changes in [`run_apply`](Self::run_apply) are persisted back
    /// to this file automatically.
    pub fn load() -> Self {
        let path = crate::state::default_path();
        App {
            targets: HashMap::new(),
            tasks: HashMap::new(),
            state: State::load(),
            state_path: Some(path),
        }
    }

    /// Create an app with a custom state file path.
    pub fn with_state_path(path: PathBuf) -> Self {
        App {
            targets: HashMap::new(),
            tasks: HashMap::new(),
            state: State::load_from(Some(path.clone())),
            state_path: Some(path),
        }
    }

    /// Register a target, or get a mutable reference to an existing one
    /// with the same name.
    pub fn target(&mut self, target: Target) -> &mut Target {
        let name = target.name.clone();
        self.targets.entry(name.clone()).or_insert(target);
        self.targets.get_mut(&name).unwrap()
    }

    /// Register a task, or get a mutable reference to an existing one
    /// with the same name.
    pub fn task(&mut self, task: Task) -> &mut Task {
        let name = task.name.clone();
        self.tasks.entry(name.clone()).or_insert(task);
        self.tasks.get_mut(&name).unwrap()
    }

    /// Look up a target by name.
    pub fn get_target(&self, name: &str) -> Option<&Target> {
        self.targets.get(name)
    }

    /// Look up a task by name.
    pub fn get_task(&self, name: &str) -> Option<&Task> {
        self.tasks.get(name)
    }

    /// Persist state to the configured state file path.
    ///
    /// Only stub targets (no check, no deps) are persisted. Leaf and
    /// aggregate target state exists only in memory during the current run.
    /// Does nothing if no state path was configured (e.g., [`App::new`]).
    pub fn save_state(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.state_path {
            let mut stub_state = State::default();
            for target in self.targets.values() {
                if target.is_stub()
                    && let Some(ts) = self.state.get(&target.name)
                {
                    stub_state.insert_entry(&target.name, ts.clone());
                }
            }
            stub_state.save_to(path)
        } else {
            Ok(())
        }
    }

    /// Iterate over all registered targets.
    pub fn all_targets(&self) -> impl Iterator<Item = &Target> {
        self.targets.values()
    }

    /// Iterate over all registered tasks.
    pub fn all_tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    /// Access the persisted state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Mutable access to the persisted state.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Validate the target/task graph.
    ///
    /// Checks:
    /// - All dependency references resolve to known targets.
    /// - All task `satisfies`/`depends_on` references resolve to known targets.
    /// - No target is satisfied by more than one task.
    /// - The dependency graph contains no cycles.
    ///
    /// Must be called before [`run_check`](Self::run_check),
    /// [`run_apply`](Self::run_apply), or [`run_plan`](Self::run_plan).
    pub fn validate(&mut self) -> anyhow::Result<()> {
        for target in self.targets.values() {
            for dep in &target.depends_on {
                if !self.targets.contains_key(dep) {
                    anyhow::bail!(
                        "target '{}' depends on unknown target '{}'",
                        target.name,
                        dep
                    );
                }
            }
            for dep in &target.check_deps {
                if !self.targets.contains_key(dep) {
                    anyhow::bail!(
                        "target '{}' has check dep on unknown target '{}'",
                        target.name,
                        dep
                    );
                }
            }
        }
        for task in self.tasks.values() {
            for sat in &task.satisfies {
                if !self.targets.contains_key(sat) {
                    anyhow::bail!("task '{}' satisfies unknown target '{}'", task.name, sat);
                }
            }
            for dep in &task.depends_on {
                if !self.targets.contains_key(dep) {
                    anyhow::bail!("task '{}' depends on unknown target '{}'", task.name, dep);
                }
            }
        }
        let mut task_targets: HashMap<&str, &str> = HashMap::new();
        for task in self.tasks.values() {
            for sat in &task.satisfies {
                if let Some(existing) = task_targets.get(sat.as_str()) {
                    anyhow::bail!(
                        "target '{}' satisfied by multiple tasks: '{}' and '{}'",
                        sat,
                        existing,
                        task.name
                    );
                }
                task_targets.insert(sat, &task.name);
            }
        }
        self.detect_cycles()?;
        Ok(())
    }

    // ── check ─────────────────────────────────────────────────────────

    /// Check target satisfaction.
    ///
    /// Runs the `check` function of each relevant target (or evaluates
    /// aggregate targets from their dependencies) and returns a topological
    /// list of results. Cached results are used when a leaf target was
    /// previously satisfied and its dependencies are unchanged.
    ///
    /// If `names` is empty, all registered targets are checked.
    /// Otherwise only the named targets (and their transitive dependencies)
    /// are included.
    pub fn run_check(&mut self, _config: &Config, names: &[String]) -> Vec<Step> {
        let order = self.topo_order(names);
        let mut results: HashMap<&str, Satisfaction> = HashMap::new();
        let mut steps = Vec::new();

        for name in &order {
            let target = &self.targets[name];
            let (sat, detail, _from_cache) = self.eval_target(target, &results);
            self.state
                .set_check_result(name, sat == Satisfaction::Satisfied);
            steps.push(self.mk_step(name, sat, detail.clone()));
            results.insert(name, sat);
        }

        let _ = self.save_state();
        steps
    }

    // ── apply ─────────────────────────────────────────────────────────

    /// Apply — check targets, run tasks for unsatisfied ones, re-check.
    ///
    /// Algorithm:
    /// 1. Collect relevant targets in topological order.
    /// 2. For each target: check → if satisfied, skip.
    /// 3. If unsatisfied and a task exists (and is not disabled by label
    ///    filter, and its own dependency targets are satisfied), run the task.
    /// 4. After a successful task, re-check all targets it satisfies.
    /// 5. If a task fails, its targets are marked as skipped and all
    ///    downstream targets are also skipped (best-effort).
    ///
    /// State is persisted automatically if a `state_path` was configured.
    pub fn run_apply(&mut self, config: &Config, names: &[String]) -> Vec<Step> {
        let order = self.topo_order(names);
        let mut sat_map: HashMap<&str, Satisfaction> = HashMap::new();
        let mut steps = Vec::new();
        let mut blocked: HashSet<&str> = HashSet::new();

        for name in &order {
            if blocked.contains(name.as_str()) {
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Unsatisfied,
                    "(skipped, upstream failure)".to_string(),
                ));
                continue;
            }

            let target = &self.targets[name];

            // Check
            let (cur_sat, cur_detail, _from_cache) = self.eval_target(target, &sat_map);
            if cur_sat == Satisfaction::Satisfied {
                self.state.set_check_result(name, true);
                steps.push(self.mk_step(name, cur_sat, cur_detail.clone()));
                sat_map.insert(name, cur_sat);
                continue;
            }

            // Stub targets (structural or demoted) cannot be satisfied by tasks
            if target.check.is_some() || target.is_stub() {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(name, Satisfaction::Unsatisfied, cur_detail));
                continue;
            }

            // Find satisfying task
            let Some(task) = self.tasks.values().find(|t| t.satisfies.contains(name)) else {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(name, Satisfaction::Unsatisfied, cur_detail));
                continue;
            };

            if self.is_task_disabled(task, config) {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Unsatisfied,
                    "(task disabled by label filter)".to_string(),
                ));
                continue;
            }

            // Check task dependency targets
            let task_deps_ok = task
                .depends_on
                .iter()
                .all(|dep| sat_map.get(dep.as_str()) == Some(&Satisfaction::Satisfied));
            if !task_deps_ok {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Unsatisfied,
                    "(task deps not satisfied)".to_string(),
                ));
                for sat in &task.satisfies {
                    blocked.insert(sat);
                }
                continue;
            }

            // Run task
            let ran_ok = match &task.run {
                Some(run) => run().is_ok(),
                None => true,
            };

            // Re-check all targets this task satisfies
            for sat_name in &task.satisfies {
                if let Some(sat_target) = self.targets.get(sat_name.as_str()) {
                    if ran_ok {
                        // Re-evaluate leaf dependencies first so aggregate targets
                        // derive from up-to-date satisfaction.
                        for dep_name in &sat_target.depends_on {
                            if let Some(dep_target) = self.targets.get(dep_name.as_str())
                                && dep_target.check.is_some()
                                && !task.satisfies.contains(dep_name)
                            {
                                let (dsat, ddetail, _) = self.eval_target(dep_target, &sat_map);
                                steps.push(self.mk_step(dep_name, dsat, ddetail));
                                sat_map.insert(dep_name, dsat);
                                self.state
                                    .set_check_result(dep_name, dsat == Satisfaction::Satisfied);
                            }
                        }
                        let (rsat, rdetail, _from_cache) = self.eval_target(sat_target, &sat_map);
                        steps.push(self.mk_step(sat_name, rsat, rdetail.clone()));
                        sat_map.insert(sat_name, rsat);
                        self.state
                            .set_check_result(sat_name, rsat == Satisfaction::Satisfied);
                    } else {
                        sat_map.insert(sat_name, Satisfaction::Unsatisfied);
                        steps.push(self.mk_step(
                            sat_name,
                            Satisfaction::Unsatisfied,
                            "(task failed)".to_string(),
                        ));
                        self.state.set_check_result(sat_name, false);
                        blocked.insert(sat_name);
                    }
                }
            }
        }

        let _ = self.save_state();
        steps
    }

    // ── plan ─────────────────────────────────────────────────────────

    /// Plan — simulate an apply run without side effects.
    ///
    /// Unlike [`run_apply`](Self::run_apply), real `check` functions are
    /// still called to show current state, but tasks are never executed.
    /// When a task *would* run, its targets are marked as
    /// "would be satisfied by task" so that downstream targets can be
    /// evaluated optimistically.
    pub fn run_plan(&self, config: &Config, names: &[String]) -> Vec<Step> {
        let order = self.topo_order(names);
        let mut sat_map: HashMap<&str, Satisfaction> = HashMap::new();
        let mut would_satisfy: HashSet<&str> = HashSet::new();
        let mut steps = Vec::new();
        let mut blocked: HashSet<&str> = HashSet::new();

        for name in &order {
            if blocked.contains(name.as_str()) {
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Unsatisfied,
                    "(would be skipped, upstream failure)".to_string(),
                ));
                continue;
            }

            // If this target would already be satisfied by a previous task, skip
            if would_satisfy.contains(name.as_str()) {
                sat_map.insert(name, Satisfaction::Satisfied);
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Satisfied,
                    "(would be satisfied by task)".to_string(),
                ));
                continue;
            }

            let target = &self.targets[name];
            let (cur_sat, cur_detail) = self.eval_target_plan(target, &sat_map, &would_satisfy);
            if cur_sat == Satisfaction::Satisfied {
                steps.push(self.mk_step(name, cur_sat, cur_detail.clone()));
                sat_map.insert(name, cur_sat);
                continue;
            }

            // Stub targets (structural or demoted) cannot be satisfied by tasks
            if target.check.is_some() || target.is_stub() {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(name, Satisfaction::Unsatisfied, cur_detail));
                continue;
            }

            // Find task
            let Some(task) = self.tasks.values().find(|t| t.satisfies.contains(name)) else {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(name, Satisfaction::Unsatisfied, cur_detail));
                continue;
            };

            if self.is_task_disabled(task, config) {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Unsatisfied,
                    "(task disabled by label filter)".to_string(),
                ));
                continue;
            }

            // Check task deps
            let task_deps_ok = task.depends_on.iter().all(|dep| {
                sat_map.get(dep.as_str()) == Some(&Satisfaction::Satisfied)
                    || would_satisfy.contains(dep.as_str())
            });
            if !task_deps_ok {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(self.mk_step(
                    name,
                    Satisfaction::Unsatisfied,
                    "(task deps not satisfied)".to_string(),
                ));
                for sat in &task.satisfies {
                    blocked.insert(sat);
                }
                continue;
            }

            // Would run this task — mark its targets as would-be-satisfied
            for sat_name in &task.satisfies {
                would_satisfy.insert(sat_name);
                sat_map.insert(sat_name, Satisfaction::Satisfied);
                steps.push(self.mk_step(
                    sat_name,
                    Satisfaction::Satisfied,
                    "(would be satisfied by task)".to_string(),
                ));
            }
        }

        steps
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn topo_order(&self, names: &[String]) -> Vec<String> {
        let mut names_set: Vec<String> = if names.is_empty() {
            self.targets.keys().cloned().collect()
        } else {
            let mut visited = HashSet::new();
            let mut stack = names.to_vec();
            while let Some(name) = stack.pop() {
                if !visited.insert(name.clone()) {
                    continue;
                }
                if let Some(t) = self.targets.get(&name) {
                    for dep in &t.depends_on {
                        if self.targets.contains_key(dep.as_str()) {
                            stack.push(dep.clone());
                        }
                    }
                    for dep in &t.check_deps {
                        if self.targets.contains_key(dep.as_str()) {
                            stack.push(dep.clone());
                        }
                    }
                }
            }
            visited.into_iter().collect()
        };

        names_set.sort();

        // Kahn's algorithm
        let mut in_deg: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        for name in &names_set {
            in_deg.entry(name.clone()).or_insert(0);
            if let Some(t) = self.targets.get(name) {
                for dep in &t.depends_on {
                    if names_set.contains(dep) {
                        adj.entry(dep.clone()).or_default().push(name.clone());
                        *in_deg.entry(name.clone()).or_insert(0) += 1;
                    }
                }
                for dep in &t.check_deps {
                    if names_set.contains(dep) {
                        adj.entry(dep.clone()).or_default().push(name.clone());
                        *in_deg.entry(name.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut queue: Vec<String> = names_set
            .iter()
            .filter(|n| in_deg.get(n.as_str()) == Some(&0))
            .cloned()
            .collect();
        queue.reverse();

        let mut order = Vec::new();
        while let Some(name) = queue.pop() {
            order.push(name.clone());
            if let Some(neighbors) = adj.get(&name) {
                for n in neighbors {
                    if let Some(d) = in_deg.get_mut(n) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push(n.clone());
                        }
                    }
                }
            }
        }

        order
    }

    fn eval_target(
        &self,
        target: &Target,
        deps_sat: &HashMap<&str, Satisfaction>,
    ) -> (Satisfaction, String, bool) {
        // Check manual override in state
        if let Some(ts) = self.state.get(&target.name)
            && ts.manually_set
        {
            let sat = match ts.satisfied {
                Some(true) => Satisfaction::Satisfied,
                _ => Satisfaction::Unsatisfied,
            };
            let detail = if sat == Satisfaction::Satisfied {
                "(manually set)"
            } else {
                "(manually unset)"
            };
            return (sat, detail.to_string(), false);
        }

        if let Some(check) = &target.check {
            // Check dependencies must be satisfied to run check
            let check_deps_ok = target
                .check_deps
                .iter()
                .all(|dep| deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied));
            if !check_deps_ok {
                return (
                    Satisfaction::Unsatisfied,
                    "(check dep not satisfied)".to_string(),
                    false,
                );
            }

            // Leaf target: use cached result if previously satisfied and check deps unchanged
            if let Some(ts) = self.state.get(&target.name)
                && ts.satisfied == Some(true)
            {
                let deps_ok = target
                    .check_deps
                    .iter()
                    .all(|dep| deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied));
                if deps_ok {
                    return (Satisfaction::Satisfied, "(cached)".to_string(), true);
                }
            }
            match check() {
                Ok(true) => (Satisfaction::Satisfied, "".to_string(), false),
                Ok(false) => (Satisfaction::Unsatisfied, String::new(), false),
                Err(e) => (
                    Satisfaction::Unsatisfied,
                    format!("check error: {e}"),
                    false,
                ),
            }
        } else {
            // Aggregate
            if target.depends_on.is_empty() {
                (
                    Satisfaction::Unsatisfied,
                    "(no check, no deps)".to_string(),
                    false,
                )
            } else if target
                .depends_on
                .iter()
                .all(|dep| deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied))
            {
                (Satisfaction::Satisfied, "(aggregate)".to_string(), false)
            } else {
                (
                    Satisfaction::Unsatisfied,
                    "(aggregate, deps unsatisfied)".to_string(),
                    false,
                )
            }
        }
    }

    fn eval_target_plan(
        &self,
        target: &Target,
        deps_sat: &HashMap<&str, Satisfaction>,
        would_satisfy: &HashSet<&str>,
    ) -> (Satisfaction, String) {
        if let Some(check) = &target.check {
            // Check dependencies must be satisfied to run check
            let check_deps_ok = target.check_deps.iter().all(|dep| {
                deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied)
                    || would_satisfy.contains(dep.as_str())
            });
            if !check_deps_ok {
                return (
                    Satisfaction::Unsatisfied,
                    "(check dep not satisfied)".to_string(),
                );
            }
            match check() {
                Ok(true) => (Satisfaction::Satisfied, "".to_string()),
                Ok(false) => (Satisfaction::Unsatisfied, String::new()),
                Err(e) => (Satisfaction::Unsatisfied, format!("check error: {e}")),
            }
        } else {
            if target.depends_on.is_empty() {
                (Satisfaction::Unsatisfied, "(no check, no deps)".to_string())
            } else if target.depends_on.iter().all(|dep| {
                deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied)
                    || would_satisfy.contains(dep.as_str())
            }) {
                (Satisfaction::Satisfied, "(aggregate)".to_string())
            } else {
                (
                    Satisfaction::Unsatisfied,
                    "(aggregate, deps unsatisfied)".to_string(),
                )
            }
        }
    }

    fn mk_step(&self, name: &str, sat: Satisfaction, detail: String) -> Step {
        let description = self.targets.get(name).and_then(|t| t.description.clone());
        Step {
            name: name.to_string(),
            description,
            sat,
            detail,
        }
    }

    fn is_task_disabled(&self, task: &Task, config: &Config) -> bool {
        for excl in &config.exclude_labels {
            if task.labels.contains(excl) {
                return true;
            }
        }
        if let Some(filter) = &config.label_filter {
            return !task.labels.iter().any(|l| filter.contains(l));
        }
        false
    }

    fn detect_cycles(&self) -> anyhow::Result<()> {
        const WHITE: u8 = 0;
        const GRAY: u8 = 1;
        const BLACK: u8 = 2;

        let mut color: HashMap<&str, u8> =
            self.targets.keys().map(|k| (k.as_str(), WHITE)).collect();
        let mut path = Vec::new();

        fn visit<'a>(
            name: &'a str,
            targets: &'a HashMap<String, Target>,
            color: &mut HashMap<&'a str, u8>,
            path: &mut Vec<&'a str>,
        ) -> anyhow::Result<()> {
            if color[name] == GRAY {
                let cycle: Vec<&str> = path.iter().skip_while(|n| **n != name).copied().collect();
                anyhow::bail!("dependency cycle: {}", cycle.join(" -> "));
            }
            if color[name] == BLACK {
                return Ok(());
            }
            color.insert(name, GRAY);
            path.push(name);
            if let Some(t) = targets.get(name) {
                for dep in &t.depends_on {
                    if targets.contains_key(dep.as_str()) {
                        visit(dep.as_str(), targets, color, path)?;
                    }
                }
                for dep in &t.check_deps {
                    if targets.contains_key(dep.as_str()) {
                        visit(dep.as_str(), targets, color, path)?;
                    }
                }
            }
            path.pop();
            color.insert(name, BLACK);
            Ok(())
        }

        let names: Vec<&str> = self.targets.keys().map(|s| s.as_str()).collect();
        for name in names {
            if color[name] == WHITE {
                visit(name, &self.targets, &mut color, &mut path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked_check() -> (std::sync::Arc<std::sync::atomic::AtomicBool>, Target) {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let c = called.clone();
        let t = Target::new("leaf").check(move || {
            c.store(true, std::sync::atomic::Ordering::SeqCst);
            anyhow::Ok(true)
        });
        (called, t)
    }

    fn make_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_cache_hit_skips_check() {
        let mut app = App::new();
        let (called, t) = tracked_check();
        app.target(t);
        app.validate().unwrap();

        // Pre-populate state as satisfied (simulating a previous run)
        app.state.set_check_result("leaf", true);

        let steps = app.run_check(&make_config(), &[]);
        assert_eq!(steps[0].sat, Satisfaction::Satisfied);
        assert!(steps[0].detail.contains("cached"));
        assert!(
            !called.load(std::sync::atomic::Ordering::SeqCst),
            "check should not have been called"
        );
    }

    #[test]
    fn test_cache_miss_check_dep_changed_runs_check() {
        let mut app = App::new();
        let (called, t) = tracked_check();
        app.target(Target::new("dep").check(|| Ok(false)));
        app.target(t.check_dep("dep"));
        app.validate().unwrap();

        // Pre-populate as satisfied
        app.state.set_check_result("leaf", true);

        let steps = app.run_check(&make_config(), &[]);
        // "dep" (alphabetically first) is unsatisfied → "leaf" check dep unsatisfied → demoted
        assert_eq!(steps[0].name, "dep");
        assert_eq!(steps[0].sat, Satisfaction::Unsatisfied);
        assert_eq!(steps[1].name, "leaf");
        assert_eq!(steps[1].sat, Satisfaction::Unsatisfied);
        assert!(
            steps[1].detail.contains("check dep not satisfied"),
            "should show check dep not satisfied, got: {}",
            steps[1].detail
        );
        assert!(
            !called.load(std::sync::atomic::Ordering::SeqCst),
            "check should NOT have been called when check dep is unsatisfied"
        );
    }

    #[test]
    fn test_cache_miss_previously_unsatisfied_runs_check() {
        let mut app = App::new();
        let (called, t) = tracked_check();
        app.target(t);
        app.validate().unwrap();

        // Pre-populate as unsatisfied
        app.state.set_check_result("leaf", false);

        let steps = app.run_check(&make_config(), &[]);
        assert_eq!(steps[0].sat, Satisfaction::Satisfied);
        assert!(
            called.load(std::sync::atomic::Ordering::SeqCst),
            "check should have been called because previous state was unsatisfied"
        );
    }

    #[test]
    fn test_aggregate_never_cached() {
        let mut app = App::new();
        app.target(Target::new("leaf").check(|| Ok(true)));
        app.target(Target::new("agg").depends_on("leaf"));
        app.validate().unwrap();

        app.state.set_check_result("leaf", true);

        let steps = app.run_check(&make_config(), &[]);
        assert_eq!(steps[0].sat, Satisfaction::Satisfied);
        assert!(steps[0].detail.contains("cached"));
        assert_eq!(steps[1].sat, Satisfaction::Satisfied);
        assert!(
            steps[1].detail.contains("aggregate"),
            "aggregate should show aggregate detail, not cached"
        );
    }

    #[test]
    fn test_check_dep_blocks_check() {
        let mut app = App::new();
        app.target(Target::new("prereq").check(|| Ok(false)));
        app.target(Target::new("main").check_dep("prereq").check(|| {
            panic!("check should never be called");
        }));
        app.validate().unwrap();

        let steps = app.run_check(&make_config(), &[]);
        // "main" > "prereq" alphabetically → prereq first
        assert_eq!(steps[0].name, "prereq");
        assert_eq!(steps[0].sat, Satisfaction::Unsatisfied);
        assert_eq!(steps[1].name, "main");
        assert_eq!(steps[1].sat, Satisfaction::Unsatisfied);
        assert!(steps[1].detail.contains("check dep not satisfied"));
    }

    #[test]
    fn test_check_dep_satisfied_runs_check() {
        let mut app = App::new();
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let c = called.clone();
        app.target(Target::new("prereq").check(|| Ok(true)));
        app.target(Target::new("main").check_dep("prereq").check(move || {
            c.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(true)
        }));
        app.validate().unwrap();

        let steps = app.run_check(&make_config(), &[]);
        // "main" > "prereq" → prereq first
        assert_eq!(steps[0].name, "prereq");
        assert_eq!(steps[0].sat, Satisfaction::Satisfied);
        assert_eq!(steps[1].name, "main");
        assert_eq!(steps[1].sat, Satisfaction::Satisfied);
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_demoted_leaf_skips_task() {
        let mut app = App::new();
        let task_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let tr = task_ran.clone();

        app.target(Target::new("net").check(|| Ok(false)));
        app.target(Target::new("rg").check_dep("net").check(|| Ok(true)));
        app.task(Task::new("install_net").satisfies("net").run(move || {
            tr.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }));

        app.validate().unwrap();
        let steps = app.run_apply(&make_config(), &[]);

        // "net" check fails → demoted to stub → no task runs
        // "rg" check dep unsatisfied → demoted to stub → no task
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].sat, Satisfaction::Unsatisfied);
        assert_eq!(steps[1].sat, Satisfaction::Unsatisfied);
        assert!(
            !task_ran.load(std::sync::atomic::Ordering::SeqCst),
            "task should NOT have run for demoted leaf"
        );
    }

    #[test]
    fn test_leaf_check_fails_no_task() {
        let mut app = App::new();
        let task_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let tr = task_ran.clone();

        app.target(Target::new("leaf").check(|| Ok(false)));
        app.task(Task::new("task").satisfies("leaf").run(move || {
            tr.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }));

        app.validate().unwrap();
        let steps = app.run_apply(&make_config(), &[]);

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].sat, Satisfaction::Unsatisfied);
        assert!(
            !task_ran.load(std::sync::atomic::Ordering::SeqCst),
            "task should NOT run for a demoted leaf"
        );
    }

    #[test]
    fn test_task_still_runs_for_aggregate() {
        let mut app = App::new();
        let task_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let tr = task_ran.clone();

        app.target(Target::new("leaf").check(|| Ok(false)));
        app.target(Target::new("agg").depends_on("leaf"));
        app.task(Task::new("task").satisfies("agg").run(move || {
            tr.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }));

        app.validate().unwrap();
        let steps = app.run_apply(&make_config(), &[]);

        assert!(task_ran.load(std::sync::atomic::Ordering::SeqCst));
        // leaf (initial) + agg (initial → task runs) + leaf (re-check dep) + agg (re-check)
        assert!(
            steps.len() == 3 || steps.len() == 4,
            "expected 3 or 4 steps, got {}",
            steps.len()
        );
        let last = steps.last().unwrap();
        assert_eq!(last.name, "agg");
        assert_eq!(last.sat, Satisfaction::Unsatisfied);
    }

    #[test]
    fn test_check_dep_cycle_detected() {
        let mut app = App::new();
        app.target(Target::new("a").check_dep("b").check(|| Ok(true)));
        app.target(Target::new("b").check_dep("a").check(|| Ok(true)));

        let err = app.validate().unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn test_unknown_check_dep_detected() {
        let mut app = App::new();
        app.target(Target::new("a").check_dep("nonexistent").check(|| Ok(true)));

        let err = app.validate().unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn test_apply_writes_state_and_caches_next_check() {
        let mut app = App::new();
        let (check_called, t) = tracked_check();
        app.target(t);
        app.task(Task::new("task").satisfies("leaf").run(|| Ok(())));
        app.validate().unwrap();

        // First apply — check runs, task runs, re-check runs
        let steps = app.run_apply(&make_config(), &[]);
        assert_eq!(steps[0].sat, Satisfaction::Satisfied);
        assert!(check_called.load(std::sync::atomic::Ordering::SeqCst));

        // Reset tracker, check again — should use cache now
        check_called.store(false, std::sync::atomic::Ordering::SeqCst);
        let steps2 = app.run_check(&make_config(), &[]);
        assert_eq!(steps2[0].sat, Satisfaction::Satisfied);
        assert!(steps2[0].detail.contains("cached"));
        assert!(!check_called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
