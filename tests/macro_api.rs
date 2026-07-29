use chezfl::Config;

#[test]
fn test_macro_basic_check() {
    let _setup = chezfl::__internals::TestSetup::new();

    chezfl::target!("base");
    chezfl::target!("ready", check: || Ok(true), depends_on: [base]);

    let mut app = chezfl::__internals::take_app();
    app.validate().unwrap();

    let config = Config::default();
    let steps = app.run_check(&config, &[]);
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].name, "base");
    assert_eq!(steps[1].name, "ready");
    assert_eq!(steps[1].sat, chezfl::Satisfaction::Satisfied);
}

#[test]
fn test_macro_apply() {
    let _setup = chezfl::__internals::TestSetup::new();

    chezfl::target!("leaf", check: || Ok(false));
    chezfl::task!("fix", satisfies: [leaf], run: || Ok(()));

    let mut app = chezfl::__internals::take_app();
    app.validate().unwrap();

    let config = Config::default();
    let steps = app.run_apply(&config, &[]);

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].sat, chezfl::Satisfaction::Unsatisfied);
}

#[test]
fn test_macro_full() {
    let _setup = chezfl::__internals::TestSetup::new();

    chezfl::target!("net");
    chezfl::target!("pkgs", depends_on: [net]);
    chezfl::target!("rg", check: || Ok(true), depends_on: [pkgs]);
    chezfl::target!("fd", check: || Ok(false), depends_on: [pkgs]);

    chezfl::task!("install_rg",
        satisfies: [rg],
        depends_on: [pkgs],
        labels: ["install"],
        run: || Ok(()),
    );

    chezfl::task!("install_fd",
        satisfies: [fd],
        depends_on: [pkgs],
        labels: ["install"],
        run: || Ok(()),
    );

    let mut app = chezfl::__internals::take_app();
    app.validate().unwrap();

    let steps = app.run_plan(&Config::default(), &[]);
    assert_eq!(steps.len(), 4);
}
