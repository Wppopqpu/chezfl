/// Register a target in the global registry.
///
/// Target names in `depends_on` are given as identifiers (not strings).
/// `stringify!` converts them to strings at compile time.
#[macro_export]
macro_rules! target {
    // Aggregate target (no check, just deps)
    ($name:expr $(,)?) => {
        $crate::__internals::register_target($crate::Target::new($name));
    };
    ($name:expr, depends_on: [$($dep:ident),* $(,)?] $(,)?) => {
        $crate::__internals::register_target({
            let mut __t = $crate::Target::new($name);
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
    ($name:expr, check: $check:expr, depends_on: [$($dep:ident),* $(,)?] $(,)?) => {
        $crate::__internals::register_target({
            let mut __t = $crate::Target::new($name).check($check);
            $(__t = __t.depends_on(stringify!($dep));)*
            __t
        });
    };
}

/// Register a task in the global registry.
#[macro_export]
macro_rules! task {
    ($name:expr, satisfies: [$($sat:ident),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name);
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
    ($name:expr, satisfies: [$($sat:ident),+ $(,)?],
     labels: [$($label:expr),+ $(,)?], run: $run:expr $(,)?) => {
        $crate::__internals::register_task({
            let mut __t = $crate::Task::new($name);
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
}

/// Build the global App from registered targets/tasks and run the CLI.
#[macro_export]
macro_rules! run {
    () => {
        let mut __app = $crate::__internals::take_app();
        $crate::run_cli(&mut __app)
    };
}
