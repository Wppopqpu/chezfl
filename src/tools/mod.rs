/// Built-in tool wrappers for common programs and filesystem operations.
///
/// Program-based modules call [`Cmd`](crate::cmd::Cmd) internally.
/// Filesystem tools ([`fs`]) use `std::fs` directly.
///
/// Designed for use inside [`Task`](crate::Task) `run` closures and
/// [`Target`](crate::Target) `check` functions.
///
/// Available tools:
/// - [`git`] — clone, pull, fetch, status, is_clean, is_git_repo, is_up_to_date_with_git
/// - [`cargo`] — install, install_path, install_root, install_force, is_installed
/// - [`yay`] — install, remove, update, is_installed
/// - [`stow`] — stow, unstow, restow, stow_everything, is_stowed, is_everything_stowed
/// - [`mime`] — xdg-mime query, is_default, set_default
/// - [`systemd`] — Systemctl builder (--user/--now), enable, daemon_reload, start, stop, is_unit_running
/// - [`fs`] — file predicates (is_file, is_dir, exists, mtime, up_to_date)
///   and operations (read, write, copy, remove, symlink)
pub mod cargo;
pub mod fs;
pub mod git;
pub mod mime;
pub mod stow;
pub mod systemd;
pub mod yay;
