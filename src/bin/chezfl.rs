use chezfl::tools::{fs, mime, yay};
use chezfl::{run_cli, App, Target, Task};
use std::path::PathBuf;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME must be set"))
}

fn register_core(app: &mut App) {
    app.target(Target::new("network").description("Network is reachable"));

    app.target(
        Target::new("pkg_yay")
            .description("yay is installed")
            .check(|| fs::is_runnable("/usr/bin/yay")),
    );
}

fn register_software(app: &mut App){
    const SOFTWARES: &[&str] = &[
        "xdg-utils",
    ];

    for &pkgname in SOFTWARES {
        app.target(
            Target::new(format!("pkg_{pkgname}"))
                .description(format!("package {pkgname} is installed"))
                .check(|| yay::is_installed(pkgname))
                .depends_on("pkg_yay")
        );
        app.task(
            Task::new(format!("install_pkg_{pkgname}"))
                .description(format!("install package {pkgname} using yay"))
                .run(|| yay::install(&[pkgname]).map(|_| ()))
                .depends_on("pkg_yay")
        );
    }
}

fn register_mime(app: &mut App) {
    const MIME: &[(&str, &str, &str, &str)] = &[
        (
            "mime_text_plain",
            "text/plain",
            "nvim.desktop",
            "text/plain defaults to nvim.desktop",
        ),
        (
            "mime_application_pdf",
            "application/pdf",
            "org.pwmt.zathura.desktop",
            "application/pdf defaults to zathura",
        ),
        (
            "mime_text_html",
            "text/html",
            "firefox.desktop",
            "text/html defaults to firefox",
        ),
    ];

    for &(name, mime_type, desktop, desc) in MIME {
        app.target(
            Target::new(name)
                .description(desc)
                .check(move || mime::is_default(mime_type, desktop)),
        );

        app.task(
            Task::new(format!("set_{name}"))
                .description(format!("run xdg-mime default for {mime_type} -> {desktop}"))
                .satisfies(name)
                .run(move || {
                    mime::set_default(mime_type, desktop)?;
                    Ok(())
                }),
        );
    }
}

fn register_niri_wants(app: &mut App) {
    let sdu = home().join(".config/systemd/user");
    let wants_dir = sdu.join("niri.service.wants");

    const NIRI_SERVICES: &[(&str, &str)] = &[
        ("noctalia", "niri.service.wants symlink for noctalia"),
        ("xrdb", "niri.service.wants symlink for xrdb"),
        ("neru", "niri.service.wants symlink for neru"),
    ];

    for &(name, desc) in NIRI_SERVICES {
        let target_name = format!("niri_wants_{name}");
        let svc_name = format!("{name}.service");

        app.target(
            Target::new(&target_name)
                .description(desc)
                .check({
                    let wants_dir = wants_dir.clone();
                    let svc_name = svc_name.clone();
                    move || fs::is_symlink(wants_dir.join(&svc_name))
                }),
        );

        app.task(
            Task::new(format!("setup_niri_wants_{name}"))
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
            .depends_on("niri_wants_noctalia")
            .depends_on("niri_wants_xrdb")
            .depends_on("niri_wants_neru"),
    );
}

fn register_koishi_cursors(app: &mut App) {
    let cursor_dir = home().join(".icons/koishi_cursors");
    let cursor_out = cursor_dir.join("cursors/text");
    let cursor_original = cursor_dir.join("original");
    let win2xcurtheme = PathBuf::from("/usr/bin/win2xcurtheme");

    app.target(
        Target::new("gui_theme_koishi_cursors")
            .description("koishi cursors are built")
            .check({
                let cursor_out = cursor_out.clone();
                move || fs::up_to_date(&cursor_out, &[&cursor_original, &win2xcurtheme])
            }),
    );

    app.task(
        Task::new("build_koishi_cursors")
            .description("build koishi cursors via fish generate.fish")
            .satisfies("gui_theme_koishi_cursors")
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
    let completion_file = home().join(".config/fish/completions/chezfl.fish");

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
            .depends_on("gui_theme_koishi_cursors"),
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
            .depends_on("cli"),
    );
}

fn main() -> anyhow::Result<()> {
    let mut app = App::load();

    register_core(&mut app);
    register_software(&mut app);
    register_mime(&mut app);
    register_niri_wants(&mut app);
    register_koishi_cursors(&mut app);
    register_shell_completions(&mut app);
    register_grouping_targets(&mut app);

    app.validate()?;
    run_cli(&mut app)
}
