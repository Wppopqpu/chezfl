use crate::tools::systemd;
use crate::{App, Target, Task, cmd, run_cmd};

const NETWORK_CONNECTIONS: &str = "nmcli";

fn non_loopback_connection_uuids() -> anyhow::Result<Vec<String>> {
    let output = run_cmd(
        NETWORK_CONNECTIONS,
        &["--terse", "--fields", "UUID,TYPE", "connection", "show"],
    )?;
    Ok(output
        .stdout
        .lines()
        .filter_map(|line| {
            let (uuid, connection_type) = line.split_once(':')?;
            (connection_type != "loopback").then(|| uuid.to_string())
        })
        .collect())
}

fn non_loopback_active_connection_uuids() -> anyhow::Result<Vec<String>> {
    let output = run_cmd(
        NETWORK_CONNECTIONS,
        &[
            "--terse",
            "--fields",
            "UUID,TYPE",
            "connection",
            "show",
            "--active",
        ],
    )?;
    Ok(output
        .stdout
        .lines()
        .filter_map(|line| {
            let (uuid, connection_type) = line.split_once(':')?;
            (connection_type != "loopback").then(|| uuid.to_string())
        })
        .collect())
}

fn nextdns_connection_settings_match(uuid: &str) -> anyhow::Result<bool> {
    let output = run_cmd(
        NETWORK_CONNECTIONS,
        &[
            "--terse",
            "--fields",
            "ipv4.ignore-auto-dns,ipv6.ignore-auto-dns",
            "connection",
            "show",
            uuid,
        ],
    )?;
    Ok(output.stdout.lines().all(|line| line.ends_with(":yes")))
}

fn register_zapret2(app: &mut App) {
    app.target(
        Target::new("zapret2.running")
            .description("zapret2 service and nftables rules are active")
            .check(|| {
                let service_running = systemd::is_unit_running("zapret2.service")?;
                cmd::request_sudo()?;

                let nft_table_exists = cmd("sudo")
                    .args(&["nft", "list", "table", "inet", "zapret2"])
                    .run()
                    .is_ok();

                Ok(service_running && nft_table_exists)
            }),
    );

    app.target(
        Target::new("zapret2")
            .description("zapret2 is running")
            .depends_on("zapret2.running"),
    );

    app.task(
        Task::new("enable_zapret2")
            .description("enable nfqws2 and start zapret2")
            .satisfies("zapret2.running")
            .depends_on("pkg.zapret2")
            .depends_on("pkg.nftables")
            .run(|| {
                systemd::enable_now("zapret2.service")?;
                Ok(())
            }),
    );
}

fn register_network(app: &mut App) {
    app.target(Target::new("network").description("network is reachable"));
    register_zapret2(app);
    register_nextdns(app);
}

fn register_nextdns(app: &mut App) {
    app.target(
        Target::new("nextdns.configured")
            .description("NextDNS DoT is configured for all non-loopback connections")
            .check(|| {
                let resolved_config = "/etc/systemd/resolved.conf.d/70-nextdns.conf";
                let network_manager_config = "/etc/NetworkManager/conf.d/20-systemd-resolved.conf";
                let resolv_target = std::fs::read_link("/etc/resolv.conf").ok();
                let connections = non_loopback_connection_uuids()?;

                Ok(!connections.is_empty()
                    && systemd::is_unit_running("systemd-resolved.service")?
                    && std::fs::read_to_string(resolved_config)
                        .unwrap_or_default()
                        .contains("DNS=45.90.28.0#b87e2c.dns.nextdns.io")
                    && std::fs::read_to_string(resolved_config)
                        .unwrap_or_default()
                        .contains("DNSOverTLS=yes")
                    && std::fs::read_to_string(network_manager_config)
                        .unwrap_or_default()
                        .contains("dns=systemd-resolved")
                    && resolv_target
                        .as_deref()
                        .is_some_and(|target| target == "/run/systemd/resolve/stub-resolv.conf")
                    && connections
                        .iter()
                        .all(|uuid| nextdns_connection_settings_match(uuid).unwrap_or(false)))
            }),
    );

    app.task(
        Task::new("enable_nextdns_resolved")
            .satisfies("nextdns.configured")
            .description("enable systemd-resolved and NextDNS DoT")
            .run(|| {
                anyhow::ensure!(
                    std::path::Path::new("/etc/systemd/resolved.conf.d/70-nextdns.conf").is_file(),
                    "deploy etcfiles/net before applying NextDNS"
                );
                anyhow::ensure!(
                    std::path::Path::new("/etc/NetworkManager/conf.d/20-systemd-resolved.conf")
                        .is_file(),
                    "deploy etcfiles/net before applying NextDNS"
                );

                cmd("sudo")
                    .args(&["systemctl", "enable", "--now", "systemd-resolved.service"])
                    .exec()?;
                cmd("sudo")
                    .args(&[
                        "ln",
                        "-sfnT",
                        "/run/systemd/resolve/stub-resolv.conf",
                        "/etc/resolv.conf",
                    ])
                    .exec()?;
                cmd("sudo").args(&["nmcli", "general", "reload"]).exec()?;

                for uuid in non_loopback_connection_uuids()? {
                    cmd("sudo")
                        .args(&[
                            "nmcli",
                            "connection",
                            "modify",
                            uuid.as_str(),
                            "ipv4.ignore-auto-dns",
                            "yes",
                            "ipv6.ignore-auto-dns",
                            "yes",
                        ])
                        .exec()?;
                }

                for uuid in non_loopback_active_connection_uuids()? {
                    cmd("sudo")
                        .args(&["nmcli", "connection", "up", "uuid", uuid.as_str()])
                        .exec()?;
                }
                Ok(())
            }),
    );
}

pub fn register(app: &mut App) {
    register_network(app);
}
