use chezfl::tools::{fs, git, mime, stow, yay};
use chezfl::{App, Target, Task, run_cli};
use std::path::PathBuf;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME must be set"))
}

/// return path related to home
fn home_path(path: &str) -> PathBuf {
    home().join(path)
}

fn register_core(app: &mut App) {
    app.target(Target::new("network").description("network is reachable"));
    app.target(Target::new("ssh_key").description("ssh keys are ready"));

    app.target(
        Target::new("pkg.yay")
            .description("yay is installed")
            .check(move || fs::is_runnable("/usr/bin/yay")),
    );

    app.target(
        Target::new("core")
            .depends_on("network")
            .depends_on("pkg.yay"),
    );
}

fn register_software(app: &mut App) {
    const SOFTWARES: &[&str] = &["git", "stow", "xdg-utils"];

    for &pkgname in SOFTWARES {
        app.target(
            Target::new(format!("pkg.{pkgname}"))
                .description(format!("package {pkgname} is installed"))
                .check_dep("pkg.yay")
                .check(move || yay::is_installed(pkgname)),
        );
        app.task(
            Task::new(format!("install_pkg.{pkgname}"))
                .description(format!("install package {pkgname} using yay"))
                .satisfies(format!("pkg.{pkgname}"))
                .run(move || yay::install(&[pkgname]).map(|_| ()))
                .depends_on("pkg.yay"),
        );
    }

    let mut pkg = Target::new("pkg");
    pkg = pkg.description("all packages is installed");
    for &pkgname in SOFTWARES {
        pkg = pkg.depends_on(format!("pkg.{pkgname}"));
    }
    pkg = pkg.depends_on("pkg.yay");
    app.target(pkg);
}

fn register_repos(app: &mut App) {
    // (name, path, url, branch)
    // path is related to ~.
    // branch: default branch to checkout
    const REPOS: &[(&str, &str, &str, &str)] = &[
        (
            "dotfiles",
            "dotfiles",
            "git@github.com:/wppopqpu/dotfiles.git",
            "main",
        ),
        (
            "etcfiles",
            "etcfiles",
            "git@github.com:wppqpqpu/etcfiles.git",
            "arch",
        ),
        (
            "crook-nvim",
            "projects/crook.nvim",
            "git@github.com:wppopqpu/crook.nvim.git",
            "main",
        ),
    ];

    let mut repo_target = Target::new("repo").description("all repos are cloned");

    for &(name, dir, url, branch) in REPOS {
        let path = home_path(dir);
        app.target(
            Target::new(format!("repo.{name}"))
                .check({
                    let path = path.clone();
                    move || git::is_git_repo(&path)
                })
                .check_dep("pkg.git")
                .description(format!("repo {name} is clonned.")),
        );

        app.task(
            Task::new(format!("clone_git_repo.{name}"))
                .satisfies(format!("repo.{name}"))
                .depends_on("pkg.git")
                .depends_on("ssh_key")
                .run({
                    let path = path.clone();
                    move || {
                        git::CloneOptions::new()
                            .all_branches()
                            .submodules()
                            .branch(branch)
                            .clone(url, &path)?;
                        Ok(())
                    }
                }),
        );

        repo_target = repo_target.depends_on(format!("repo.{name}"));
    }

    app.target(repo_target);
}

fn register_stow(app: &mut App) {
    // (name, from, to)
    const SPEC: &[(&str, &str, &str)] = &[
        ("dotfiles", "dotfiles", ""),
        ("local", ".local/stow", ".local"),
    ];

    let mut target_stow = Target::new("stow").description("all repos are stowed");

    for &(name, from, to) in SPEC {
        let from = home_path(from);
        let to = home_path(to);

        app.target(
            Target::new(format!("stow.{name}"))
                .check({
                    let from = from.clone();
                    let to = to.clone();
                    move || stow::is_everything_stowed(&from, &to)
                })
                .description(format!("repo {name} is stowed")),
        );

        app.task(
            Task::new(format!("stow.{name}"))
                .satisfies(format!("stow.{name}"))
                .run(move || stow::stow_everything(&from, &to))
                .depends_on("pkg.stow"),
        );

        target_stow = target_stow.depends_on(format!("stow.{name}"));
    }
    app.target(target_stow);
}

fn register_mime(app: &mut App) {
    const MIME: &[(&str, &str, &str, &str)] = &[
        (
            "text_plain",
            "text/plain",
            "nvim.desktop",
            "text/plain defaults to nvim.desktop",
        ),
        (
            "application_pdf",
            "application/pdf",
            "org.pwmt.zathura.desktop",
            "application/pdf defaults to zathura",
        ),
        (
            "text_html",
            "text/html",
            "firefox.desktop",
            "text/html defaults to firefox",
        ),
    ];

    for &(name, mime_type, desktop, desc) in MIME {
        app.target(
            Target::new(format!("mime.{name}"))
                .description(desc)
                .check(move || mime::is_default(mime_type, desktop))
                .check_dep("pkg.xdg-utils"),
        );

        app.task(
            Task::new(format!("set_mime.{name}"))
                .description(format!("run xdg-mime default for {mime_type} -> {desktop}"))
                .satisfies(format!("mime.{name}"))
                .run(move || {
                    mime::set_default(mime_type, desktop)?;
                    Ok(())
                })
                .depends_on("pkg.xdg-utils"),
        );
    }

    let mut mime = Target::new("mime");
    mime = mime.description("all packages is installed");
    for &(name, _, _, _) in MIME {
        mime = mime.depends_on(format!("mime.{name}"));
    }
    app.target(mime);
}

fn register_niri_wants(app: &mut App) {
    let sdu = home_path(".config/systemd/user");
    let wants_dir = sdu.join("niri.service.wants");

    const NIRI_SERVICES: &[(&str, &str)] = &[
        ("noctalia", "niri.service.wants symlink for noctalia"),
        ("xrdb", "niri.service.wants symlink for xrdb"),
        ("neru", "niri.service.wants symlink for neru"),
    ];

    for &(name, desc) in NIRI_SERVICES {
        let target_name = format!("niri_wants.{name}");
        let svc_name = format!("{name}.service");

        app.target(Target::new(&target_name).description(desc).check({
            let wants_dir = wants_dir.clone();
            let svc_name = svc_name.clone();
            move || fs::is_symlink(wants_dir.join(&svc_name))
        }));

        app.task(
            Task::new(format!("setup_niri_wants.{name}"))
                .description(format!("symlink {name}.service into niri.service.wants"))
                .satisfies(&target_name)
                .run({
                    let wants_dir = wants_dir.clone();
                    let sdu = sdu.clone();
                    move || {
                        let dst = wants_dir.join(&svc_name);
                        let _ = std::fs::remove_file(&dst);
                        fs::symlink(sdu.join(&svc_name), dst)
                    }
                }),
        );
    }

    app.target(
        Target::new("niri_wants")
            .description("all niri.service.wants symlinks")
            .depends_on("niri_wants.noctalia")
            .depends_on("niri_wants.xrdb")
            .depends_on("niri_wants.neru"),
    );
}

fn register_koishi_cursors(app: &mut App) {
    let cursor_dir = home_path(".icons/koishi_cursors");
    let cursor_out = cursor_dir.join("cursors/text");
    let cursor_original = cursor_dir.join("original");
    let win2xcurtheme = PathBuf::from("/usr/bin/win2xcurtheme");

    app.target(
        Target::new("gui_theme.koishi_cursors")
            .description("koishi cursors are built")
            .check({
                let cursor_out = cursor_out.clone();
                move || fs::up_to_date(&cursor_out, &[&cursor_original, &win2xcurtheme])
            }),
    );

    app.task(
        Task::new("build_koishi_cursors")
            .description("build koishi cursors via fish generate.fish")
            .satisfies("gui_theme.koishi_cursors")
            .run(move || {
                let status = std::process::Command::new("fish")
                    .arg("generate.fish")
                    .current_dir(&cursor_dir)
                    .status()?;
                anyhow::ensure!(status.success(), "fish generate.fish failed");
                Ok(())
            }),
    );
}

fn register_shell_completions(app: &mut App) {
    let binary = std::env::current_exe().expect("failed to get current exe path");
    let completion_file = home_path(".config/fish/completions/chezfl.fish");

    app.target(
        Target::new("chezfl_fish_completions")
            .description("fish completion script for chezfl is up-to-date")
            .check({
                let completion_file = completion_file.clone();
                let binary = binary.clone();
                move || fs::up_to_date(&completion_file, &[&binary])
            }),
    );

    app.task(
        Task::new("generate_chezfl_fish_completions")
            .description("generate fish completion script for chezfl")
            .satisfies("chezfl_fish_completions")
            .run(move || {
                let output = std::process::Command::new(&binary)
                    .env("COMPLETE", "fish")
                    .output()?;
                anyhow::ensure!(output.status.success(), "completion generation failed");
                fs::write(&completion_file, String::from_utf8(output.stdout)?.as_str())
            }),
    );

    app.target(
        Target::new("cli_shell_completions")
            .description("shell completions for installed tools")
            .depends_on("chezfl_fish_completions"),
    );
}

fn register_grouping_targets(app: &mut App) {
    app.target(
        Target::new("install_systemd_units")
            .description("all systemd unit setups")
            .depends_on("niri_wants"),
    );

    app.target(
        Target::new("gui_theme")
            .description("GUI theme setup")
            .depends_on("gui_theme.koishi_cursors"),
    );

    app.target(
        Target::new("gui")
            .description("GUI setup")
            .depends_on("gui_theme"),
    );

    app.target(
        Target::new("cli")
            .description("CLI setup")
            .depends_on("cli_shell_completions"),
    );

    app.target(
        Target::new("all")
            .description("everything")
            .depends_on("install_systemd_units")
            .depends_on("gui")
            .depends_on("cli")
            .depends_on("pkg")
            .depends_on("mime")
            .depends_on("core")
            .depends_on("repo")
            .depends_on("stow"),
    );
}

fn main() -> anyhow::Result<()> {
    let mut app = App::load();

    register_core(&mut app);
    register_software(&mut app);
    register_repos(&mut app);
    register_stow(&mut app);
    register_mime(&mut app);
    register_niri_wants(&mut app);
    register_koishi_cursors(&mut app);
    register_shell_completions(&mut app);
    register_grouping_targets(&mut app);

    app.validate()?;
    run_cli(&mut app)
}
