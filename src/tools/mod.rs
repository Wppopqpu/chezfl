/// Built-in tool wrappers for common programs.
///
/// Each module contains free functions that call [`Cmd`](crate::cmd::Cmd)
/// internally. Designed for use inside [`Task`](crate::Task) `run` closures.
///
/// Available tools:
/// - [`git`] — clone, pull, fetch, status
/// - [`yay`] — install, remove, update, is_installed
pub mod git;
pub mod yay;
