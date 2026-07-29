use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::Satisfaction;
use crate::Step;
use crate::state::State;
use crate::target::Target;
use crate::task::Task;

#[derive(Clone, Default)]
pub struct Config {
    pub label_filter: Option<Vec<String>>,
    pub exclude_labels: Vec<String>,
}

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
    pub fn new() -> Self {
        App {
            targets: HashMap::new(),
            tasks: HashMap::new(),
            state: State::default(),
            state_path: None,
        }
    }

    pub fn load() -> Self {
        let path = crate::state::default_path();
        App {
            targets: HashMap::new(),
            tasks: HashMap::new(),
            state: State::load(),
            state_path: Some(path),
        }
    }

    pub fn with_state_path(path: PathBuf) -> Self {
        App {
            targets: HashMap::new(),
            tasks: HashMap::new(),
            state: State::load_from(Some(path.clone())),
            state_path: Some(path),
        }
    }

    pub fn target(&mut self, target: Target) -> &mut Target {
        let name = target.name.clone();
        self.targets.entry(name.clone()).or_insert(target);
        self.targets.get_mut(&name).unwrap()
    }

    pub fn task(&mut self, task: Task) -> &mut Task {
        let name = task.name.clone();
        self.tasks.entry(name.clone()).or_insert(task);
        self.tasks.get_mut(&name).unwrap()
    }

    pub fn get_target(&self, name: &str) -> Option<&Target> {
        self.targets.get(name)
    }

    pub fn get_task(&self, name: &str) -> Option<&Task> {
        self.tasks.get(name)
    }

    pub fn all_targets(&self) -> impl Iterator<Item = &Target> {
        self.targets.values()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Validate the graph. Must be called before run_*.
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

    /// Check targets, return a topological list of steps.
    pub fn run_check(&self, _config: &Config, names: &[String]) -> Vec<Step> {
        let order = self.topo_order(names);
        let mut results: HashMap<&str, Satisfaction> = HashMap::new();
        let mut steps = Vec::new();

        for name in &order {
            let target = &self.targets[name];
            let (sat, detail) = self.eval_target(target, &results);
            steps.push(Step {
                name: name.clone(),
                sat,
                detail: detail.clone(),
            });
            results.insert(name, sat);
        }

        steps
    }

    // ── apply ─────────────────────────────────────────────────────────

    /// Apply: check → run tasks for unsatisfied → recheck.
    pub fn run_apply(&mut self, config: &Config, names: &[String]) -> Vec<Step> {
        let order = self.topo_order(names);
        let mut sat_map: HashMap<&str, Satisfaction> = HashMap::new();
        let mut steps = Vec::new();
        let mut blocked: HashSet<&str> = HashSet::new();

        for name in &order {
            if blocked.contains(name.as_str()) {
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: "(skipped, upstream failure)".to_string(),
                });
                continue;
            }

            let target = &self.targets[name];

            // Check
            let (cur_sat, cur_detail) = self.eval_target(target, &sat_map);
            if cur_sat == Satisfaction::Satisfied {
                steps.push(Step {
                    name: name.clone(),
                    sat: cur_sat,
                    detail: cur_detail.clone(),
                });
                sat_map.insert(name, cur_sat);
                continue;
            }

            // Find satisfying task
            let Some(task) = self.tasks.values().find(|t| t.satisfies.contains(name)) else {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: cur_detail,
                });
                continue;
            };

            if self.is_task_disabled(task, config) {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: "(task disabled by label filter)".to_string(),
                });
                continue;
            }

            // Check task dependency targets
            let task_deps_ok = task
                .depends_on
                .iter()
                .all(|dep| sat_map.get(dep.as_str()) == Some(&Satisfaction::Satisfied));
            if !task_deps_ok {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: "(task deps not satisfied)".to_string(),
                });
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
                        let (rsat, rdetail) = self.eval_target(sat_target, &sat_map);
                        steps.push(Step {
                            name: sat_name.clone(),
                            sat: rsat,
                            detail: rdetail.clone(),
                        });
                        sat_map.insert(sat_name, rsat);
                        self.state
                            .set_check_result(sat_name, rsat == Satisfaction::Satisfied);
                    } else {
                        sat_map.insert(sat_name, Satisfaction::Unsatisfied);
                        steps.push(Step {
                            name: sat_name.clone(),
                            sat: Satisfaction::Unsatisfied,
                            detail: "(task failed)".to_string(),
                        });
                        self.state.set_check_result(sat_name, false);
                        blocked.insert(sat_name);
                    }
                }
            }
        }

        if let Some(path) = &self.state_path {
            let _ = self.state.save_to(path);
        }
        steps
    }

    // ── plan ─────────────────────────────────────────────────────────

    /// Plan: simulate apply without running tasks.
    pub fn run_plan(&self, config: &Config, names: &[String]) -> Vec<Step> {
        let order = self.topo_order(names);
        let mut sat_map: HashMap<&str, Satisfaction> = HashMap::new();
        let mut would_satisfy: HashSet<&str> = HashSet::new();
        let mut steps = Vec::new();
        let mut blocked: HashSet<&str> = HashSet::new();

        for name in &order {
            if blocked.contains(name.as_str()) {
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: "(would be skipped, upstream failure)".to_string(),
                });
                continue;
            }

            // If this target would already be satisfied by a previous task, skip
            if would_satisfy.contains(name.as_str()) {
                sat_map.insert(name, Satisfaction::Satisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Satisfied,
                    detail: "(would be satisfied by task)".to_string(),
                });
                continue;
            }

            let target = &self.targets[name];
            let (cur_sat, cur_detail) = self.eval_target_plan(target, &sat_map, &would_satisfy);
            if cur_sat == Satisfaction::Satisfied {
                steps.push(Step {
                    name: name.clone(),
                    sat: cur_sat,
                    detail: cur_detail.clone(),
                });
                sat_map.insert(name, cur_sat);
                continue;
            }

            // Find task
            let Some(task) = self.tasks.values().find(|t| t.satisfies.contains(name)) else {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: cur_detail,
                });
                continue;
            };

            if self.is_task_disabled(task, config) {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: "(task disabled by label filter)".to_string(),
                });
                continue;
            }

            // Check task deps
            let task_deps_ok = task.depends_on.iter().all(|dep| {
                sat_map.get(dep.as_str()) == Some(&Satisfaction::Satisfied)
                    || would_satisfy.contains(dep.as_str())
            });
            if !task_deps_ok {
                sat_map.insert(name, Satisfaction::Unsatisfied);
                steps.push(Step {
                    name: name.clone(),
                    sat: Satisfaction::Unsatisfied,
                    detail: "(task deps not satisfied)".to_string(),
                });
                for sat in &task.satisfies {
                    blocked.insert(sat);
                }
                continue;
            }

            // Would run this task — mark its targets as would-be-satisfied
            for sat_name in &task.satisfies {
                would_satisfy.insert(sat_name);
                sat_map.insert(sat_name, Satisfaction::Satisfied);
                steps.push(Step {
                    name: sat_name.clone(),
                    sat: Satisfaction::Satisfied,
                    detail: "(would be satisfied by task)".to_string(),
                });
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
    ) -> (Satisfaction, String) {
        // Check manual override in state
        if let Some(ts) = self.state.get(&target.name)
            && ts.manually_set
        {
            return match ts.satisfied {
                Some(true) => (Satisfaction::Satisfied, "(manually set)".to_string()),
                _ => (Satisfaction::Unsatisfied, "(manually unset)".to_string()),
            };
        }

        if let Some(check) = &target.check {
            match check() {
                Ok(true) => (Satisfaction::Satisfied, "".to_string()),
                Ok(false) => (Satisfaction::Unsatisfied, "".to_string()),
                Err(e) => (Satisfaction::Unsatisfied, format!("check error: {e}")),
            }
        } else {
            // Aggregate
            let all_ok = target
                .depends_on
                .iter()
                .all(|dep| deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied));
            if all_ok {
                (Satisfaction::Satisfied, "(aggregate)".to_string())
            } else {
                (
                    Satisfaction::Unsatisfied,
                    "(aggregate, deps unsatisfied)".to_string(),
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
            match check() {
                Ok(true) => (Satisfaction::Satisfied, "".to_string()),
                Ok(false) => (Satisfaction::Unsatisfied, "".to_string()),
                Err(e) => (Satisfaction::Unsatisfied, format!("check error: {e}")),
            }
        } else {
            let all_ok = target.depends_on.iter().all(|dep| {
                deps_sat.get(dep.as_str()) == Some(&Satisfaction::Satisfied)
                    || would_satisfy.contains(dep.as_str())
            });
            if all_ok {
                (Satisfaction::Satisfied, "(aggregate)".to_string())
            } else {
                (
                    Satisfaction::Unsatisfied,
                    "(aggregate, deps unsatisfied)".to_string(),
                )
            }
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
