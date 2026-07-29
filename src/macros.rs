/// Declare a target via the global registry.
///
/// Target names in `depends_on` are given as **identifiers** (not strings).
/// `stringify!` converts them to strings at compile time.
///
/// # Examples
///
/// ```ignore
/// // Aggregate target (no check, satisfaction from deps)
/// target!("system_ready", depends_on: [apt, network]);
///
/// // Leaf target with check
/// target!("rg_installed",
///     check: || Ok(Command::new("which").arg("rg").status()?.success()),
///     depends_on: [apt],
/// );
/// ```
#[macro_export]
macro_rules! target {
    // Aggregate target (no check, just deps)
    ($name:expr $(,)?) => {
        $crate::__internals::register_target($crate::Target::new($name));
    };
    ($name:expr, description: $desc:expr $(,)?) => {
        $crate::__internals::register_target(
            $crate::Target::new($name).description($desc),
        );
    };
    ($name:expr, depends_on: [$($dep:ident),* $(,)?] $(,)?) => {
        $crate::__internals::register_target({
            let mut __t = $crate::Target::new($name);
            $(__t = __t.depends_on(stringify!($dep));)*
            __t
        });
    };
    ($name:expr, description: $desc:expr, depends_on: [$($dep:ident),* $(,)?] $(,)?) => {
        $crate::__internals::register_target({
            let mut __t = $crate::Target::new($name).description($desc);
            $(__t = __t.depends_on(stringify!($dep));)*
            __t
        });
    };
    // Leaf target with check
    ($name:expr, check: $check:expr $(,)?) => {
        $crate::__internals::register_target(
            $crate::Target::new($name).check($check),
        );
    };
    ($name:expr, description: $desc:expr, check: $check:expr $(,)?) => {
        $crate::__internals::register_target(
            $crate::Target::new($name).description($desc).check($check),
        );
    };
    ($name:expr, check: $check:expr, depends_on: [$($dep:ident),* $(,)?] $(,)?) => {
        $crate::__internals::register_target({
            let mut __t = $crate::Target::new($name).check($check);
            $(__t = __t.depends_on(stringify!($dep));)*
            __t
        });
    };
    ($name:expr, description: $desc:expr, check: $check:expr, depends_on: [$($dep:ident),* $(,)?] $(,)?) => {
        $crate::__internals::register_target({
            let mut __t = $crate::Target::new($name).description($desc).check($check);
            $(__t = __t.depends_on(stringify!($dep));)*
            __t
        });
    };
}

/// Declare a task via the global registry.
///
/// Target references in `satisfies` and `depends_on` are given as
/// **identifiers** (not strings). `stringify!` converts them at compile time.
///
/// # Examples
///
/// ```ignore
/// task!("install_rg",
///     satisfies: [rg_installed],
///     depends_on: [apt_ready],
///     labels: ["install"],
///     run: || {
///         Command::new("sudo").args(["pacman", "-S", "ripgrep"]).status()?;
///         Ok(())
///     },
/// );
/// ```
#[macro_export]
macro_rules! task {
    ($name:expr, satisfies: [$($sat:ident),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name);
            $(__t = __t.satisfies(stringify!($sat));)+
            __t.run($run)
        });
    };
    ($name:expr, description: $desc:expr, satisfies: [$($sat:ident),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name).description($desc);
            $(__t = __t.satisfies(stringify!($sat));)+
            __t.run($run)
        });
    };
    ($name:expr, satisfies: [$($sat:ident),+ $(,)?],
     depends_on: [$($dep:ident),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name);
            $(__t = __t.satisfies(stringify!($sat));)+
            $(__t = __t.depends_on(stringify!($dep));)+
            __t.run($run)
        });
    };
    ($name:expr, description: $desc:expr, satisfies: [$($sat:ident),+ $(,)?],
     depends_on: [$($dep:ident),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name).description($desc);
            $(__t = __t.satisfies(stringify!($sat));)+
            $(__t = __t.depends_on(stringify!($dep));)+
            __t.run($run)
        });
    };
    ($name:expr, satisfies: [$($sat:ident),+ $(,)?],
     labels: [$($label:expr),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name);
            $(__t = __t.satisfies(stringify!($sat));)+
            $(__t = __t.label($label);)+
            __t.run($run)
        });
    };
    ($name:expr, description: $desc:expr, satisfies: [$($sat:ident),+ $(,)?],
     labels: [$($label:expr),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name).description($desc);
            $(__t = __t.satisfies(stringify!($sat));)+
            $(__t = __t.label($label);)+
            __t.run($run)
        });
    };
    ($name:expr, satisfies: [$($sat:ident),+ $(,)?],
     depends_on: [$($dep:ident),+ $(,)?],
     labels: [$($label:expr),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name);
            $(__t = __t.satisfies(stringify!($sat));)+
            $(__t = __t.depends_on(stringify!($dep));)+
            $(__t = __t.label($label);)+
            __t.run($run)
        });
    };
    ($name:expr, description: $desc:expr, satisfies: [$($sat:ident),+ $(,)?],
     depends_on: [$($dep:ident),+ $(,)?],
     labels: [$($label:expr),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name).description($desc);
            $(__t = __t.satisfies(stringify!($sat));)+
            $(__t = __t.depends_on(stringify!($dep));)+
            $(__t = __t.label($label);)+
            __t.run($run)
        });
    };
}

/// Build the global [`App`](crate::App) from registered targets/tasks
/// and run the CLI.
///
/// Must be called after all [`target!`] and [`task!`] declarations.
/// Expands to:
///
/// ```ignore
/// let mut __app = chezfl::__internals::take_app();
/// chezfl::run_cli(&mut __app)
/// ```
///
/// # Example
///
/// ```ignore
/// target!("base");
/// task!("do_stuff", satisfies: [base], run: || Ok(()));
/// run!();  // parses argv, runs apply/check/plan
/// ```
#[macro_export]
macro_rules! run {
    () => {{
        let mut __app = $crate::__internals::take_app();
        $crate::run_cli(&mut __app)
    }};
}
