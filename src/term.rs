use std::io::IsTerminal;

fn paint(code: &str, s: &str) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub(crate) fn green(s: &str) -> String {
    paint("32", s)
}

pub(crate) fn red(s: &str) -> String {
    paint("31", s)
}

pub(crate) fn yellow(s: &str) -> String {
    paint("33", s)
}

pub(crate) fn dim(s: &str) -> String {
    paint("2", s)
}

pub(crate) fn red_strike(s: &str) -> String {
    paint("31;9", s)
}

pub(crate) fn bold(s: &str) -> String {
    paint("1", s)
}
