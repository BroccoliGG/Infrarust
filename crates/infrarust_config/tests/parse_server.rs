#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use infrarust_config::{
    ProxyMode, ServerConfig, ServerManagerConfig, motd_warnings, validate_server_config,
};

fn load_survival() -> ServerConfig {
    let toml_str = include_str!("fixtures/survival.toml");
    toml::from_str(toml_str).expect("failed to parse survival.toml")
}

fn load_creative() -> ServerConfig {
    let toml_str = include_str!("fixtures/creative.toml");
    toml::from_str(toml_str).expect("failed to parse creative.toml")
}

#[test]
fn test_parse_survival_domains() {
    let config = load_survival();

    assert_eq!(config.domains.len(), 2);
    assert_eq!(config.domains[0], "survival.mc.example.com");
    assert_eq!(config.domains[1], "*.survival.example.com");
}

#[test]
fn test_parse_survival_addresses() {
    let config = load_survival();

    assert_eq!(config.addresses.len(), 2);
    assert_eq!(config.addresses[0].address.host, "10.0.1.10");
    assert_eq!(config.addresses[0].address.port, 25565);
    assert_eq!(config.addresses[1].address.host, "10.0.1.11");
    assert_eq!(config.addresses[1].address.port, 25565);
}

#[test]
fn test_parse_survival_proxy_mode() {
    let config = load_survival();
    assert_eq!(config.proxy_mode, ProxyMode::Passthrough);
}

#[test]
fn test_parse_survival_motd() {
    let config = load_survival();

    let online = config
        .motd
        .online
        .as_ref()
        .expect("motd.online should be set");
    assert_eq!(online.text, "§aSurvival §7— §fBienvenue !");
    assert_eq!(online.favicon.as_deref(), Some("./icons/survival.png"));

    let sleeping = config
        .motd
        .sleeping
        .as_ref()
        .expect("motd.sleeping should be set");
    assert_eq!(sleeping.version_name.as_deref(), Some("Server Sleeping"));

    assert!(config.motd.starting.is_some());
}

#[test]
fn test_parse_survival_server_manager() {
    let config = load_survival();

    match config.server_manager {
        Some(ServerManagerConfig::Pterodactyl(ref ptero)) => {
            assert_eq!(ptero.api_url, "https://panel.example.com");
            assert_eq!(ptero.api_key, "ptlc_xxxxx");
            assert_eq!(ptero.server_id, "abc123");
        }
        other => panic!("expected Pterodactyl, got {other:?}"),
    }
}

#[test]
fn test_parse_survival_max_players() {
    let config = load_survival();
    assert_eq!(config.max_players, 100);
}

#[test]
fn test_parse_creative_proxy_mode() {
    let config = load_creative();
    assert_eq!(config.proxy_mode, ProxyMode::ClientOnly);
}

#[test]
fn test_parse_creative_server_manager_local() {
    let config = load_creative();

    match config.server_manager {
        Some(ServerManagerConfig::Local(ref local)) => {
            assert_eq!(local.command, "java");
            assert_eq!(
                local.working_dir.to_str().unwrap(),
                "/opt/minecraft/creative"
            );
            assert_eq!(local.args, vec!["-Xmx4G", "-jar", "server.jar", "nogui"]);
        }
        other => panic!("expected Local, got {other:?}"),
    }
}

#[test]
fn test_parse_motd_version_protocol() {
    let toml_str = r#"
        domains = ["test.example.com"]
        addresses = ["127.0.0.1:25565"]

        [motd.online]
        text = "Legacy"
        version_name = "1.8.x"
        version_protocol = 47
    "#;
    let config: ServerConfig = toml::from_str(toml_str).expect("failed to parse version_protocol");
    let online = config
        .motd
        .online
        .as_ref()
        .expect("motd.online should be set");
    assert_eq!(online.version_protocol, Some(47));
    assert!(config.motd.sleeping.is_none());
}

/// `offline` is no longer a MOTD state, but the docs once showed
/// `[motd.offline]`, so server files written from them must keep loading.
#[test]
fn test_parse_motd_offline_is_ignored() {
    let toml_str = r#"
        domains = ["test.example.com"]
        addresses = ["127.0.0.1:25565"]

        [motd.online]
        text = "Online"

        [motd.offline]
        text = "Offline"
    "#;
    let config: ServerConfig = toml::from_str(toml_str).expect("[motd.offline] should still parse");
    let online = config
        .motd
        .online
        .as_ref()
        .expect("motd.online should be set");
    assert_eq!(online.text, "Online");
    assert!(config.motd.sleeping.is_none());
    assert!(config.motd.unreachable.is_none());

    let warnings = motd_warnings(&config);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("[motd.offline]"));
    assert!(validate_server_config(&config).is_ok());

    let serialized = toml::to_string(&config).expect("serialize");
    assert!(
        !serialized.contains("offline"),
        "the ignored entry must not be written back:\n{serialized}"
    );

    assert!(motd_warnings(&load_survival()).is_empty());
}

#[test]
fn test_unknown_motd_state_is_rejected() {
    let toml_str = r#"
        domains = ["test.example.com"]
        addresses = ["127.0.0.1:25565"]

        [motd.maintenance]
        text = "Down for maintenance"
    "#;
    let result: Result<ServerConfig, _> = toml::from_str(toml_str);
    assert!(result.is_err(), "unknown MOTD state should cause an error");
}

#[test]
fn test_deny_unknown_fields() {
    let toml_str = r#"
        domains = ["test.example.com"]
        addresses = ["127.0.0.1:25565"]
        unknown_field = "oops"
    "#;
    let result: Result<ServerConfig, _> = toml::from_str(toml_str);
    assert!(result.is_err(), "unknown field should cause an error");
}
