use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chezfl::{App, Config, Satisfaction, Target, Task};

#[test]
fn test_check_leaf_target() {
    let mut app = App::new();
    app.target(Target::new("a_ok").check(|| Ok(true)));
    app.target(Target::new("b_fail").check(|| Ok(false)));
    app.validate().unwrap();

    let config = Config::default();
    let steps = app.run_check(&config, &[]);
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].sat, Satisfaction::Satisfied);
    assert_eq!(steps[1].sat, Satisfaction::Unsatisfied);
}

#[test]
fn test_aggregate_target() {
    let mut app = App::new();
    app.target(Target::new("leaf").check(|| Ok(true)));
    app.target(Target::new("agg").depends_on("leaf"));

    app.validate().unwrap();
    let config = Config::default();
    let steps = app.run_check(&config, &[]);

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].sat, Satisfaction::Satisfied); // leaf
    assert_eq!(steps[1].sat, Satisfaction::Satisfied); // agg
}

#[test]
fn test_apply_runs_task() {
    let mut app = App::new();
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    // Aggregate targets (no check) can be satisfied by tasks
    app.target(Target::new("agg").depends_on("leaf"));
    app.target(Target::new("leaf").check(|| Ok(false)));
    app.task(Task::new("task").satisfies("agg").run(move || {
        ran_clone.store(true, Ordering::SeqCst);
        Ok(())
    }));

    app.validate().unwrap();
    let config = Config::default();
    let steps = app.run_apply(&config, &[]);

    assert!(ran.load(Ordering::SeqCst), "task should have run");
    // leaf (initial) + agg (initial, task ran) + leaf (re-check dep) + agg (re-check)
    assert!(steps.len() >= 3, "expected 3+ steps, got {}", steps.len());
}

#[test]
fn test_target_deps_not_duplicated_in_topo_order() {
    let mut app = App::new();
    app.target(Target::new("base").check(|| Ok(true)));
    app.target(Target::new("mid").depends_on("base").check(|| Ok(true)));
    app.target(Target::new("top").depends_on("mid"));

    app.validate().unwrap();
    let config = Config::default();
    let steps = app.run_check(&config, &[]);

    assert_eq!(steps.len(), 3);
    // base first, then mid, then top
    assert_eq!(steps[0].name, "base");
    assert_eq!(steps[1].name, "mid");
    assert_eq!(steps[2].name, "top");
}

#[test]
fn test_label_filter_disables_task() {
    let mut app = App::new();
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    app.target(Target::new("agg").depends_on("leaf"));
    app.target(Target::new("leaf").check(|| Ok(false)));
    app.task(
        Task::new("task")
            .satisfies("agg")
            .label("slow")
            .run(move || {
                ran_clone.store(true, Ordering::SeqCst);
                Ok(())
            }),
    );

    app.validate().unwrap();
    let config = Config {
        label_filter: None,
        exclude_labels: vec!["slow".to_string()],
    };
    let steps = app.run_apply(&config, &[]);

    assert!(!ran.load(Ordering::SeqCst), "task should NOT have run");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].sat, Satisfaction::Unsatisfied);
    assert_eq!(steps[1].sat, Satisfaction::Unsatisfied);
}

#[test]
fn test_plan_shows_what_would_happen() {
    let mut app = App::new();
    // Aggregate targets can be satisfied by tasks in plan mode
    app.target(Target::new("agg").depends_on("leaf"));
    app.target(Target::new("leaf").check(|| Ok(false)));
    app.task(Task::new("task").satisfies("agg").run(|| Ok(())));

    app.validate().unwrap();
    let config = Config::default();
    let steps = app.run_plan(&config, &[]);

    // In plan mode: leaf check fails (demoted), agg depends on leaf → agg unsatisfied → task would run
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].name, "leaf");
    assert_eq!(steps[0].sat, Satisfaction::Unsatisfied);
    assert_eq!(steps[1].name, "agg");
    assert_eq!(steps[1].sat, Satisfaction::Satisfied);
}

#[test]
fn test_validate_unknown_dep() {
    let mut app = App::new();
    app.target(Target::new("leaf").depends_on("nonexistent"));

    let err = app.validate().unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}

#[test]
fn test_validate_cycle() {
    let mut app = App::new();
    app.target(Target::new("a").depends_on("b"));
    app.target(Target::new("b").depends_on("a"));

    let err = app.validate().unwrap_err();
    assert!(err.to_string().contains("cycle"));
}
