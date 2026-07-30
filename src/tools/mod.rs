/// Built-in tool wrappers for common programs and filesystem operations.
///
/// Program-based modules call [`Cmd`](crate::cmd::Cmd) internally.
/// Filesystem tools ([`fs`]) use `std::fs` directly.
///
/// Designed for use inside [`Task`](crate::Task) `run` closures and
/// [`Target`](crate::Target) `check` functions.
///
/// Available tools:
/// - [`git`] — clone, pull, fetch, status
/// - [`yay`] — install, remove, update, is_installed
/// - [`mime`] — xdg-mime query, is_default, set_default
/// - [`fs`] — file predicates (is_file, is_dir, exists, mtime, up_to_date)
///   and operations (read, write, copy, remove, symlink)
pub mod fs;
pub mod git;
pub mod mime;
pub mod yay;
