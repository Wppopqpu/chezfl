# Configuration as Rust Code, Not a DSL

Users declare targets and tasks by calling chezfl's Rust API directly. There is no separate config language, YAML, TOML, or DSL parser.

**Context**: chezfl could have taken a traditional approach — define targets in TOML/YAML, or create a small declarative DSL like Nix. Instead, it leans into Rust itself as the declaration language.

**Why**: A DSL means maintaining a parser, a type-checker, and a standard library. Rust already provides loops, conditionals, error handling, type safety, and the entire crate ecosystem. Users who want to iterate over a list of repos to generate targets just write a `for` loop. Users who want to skip targets on certain machines check `std::env::consts::HOST`. No abstraction layer stands between the user and the full power of the host language.

**Trade-off**: The user must write Rust. This is not a config-tool-for-non-programmers — it assumes the user is comfortable writing and compiling Rust code. That is an acceptable constraint for a personal system state manager.
