use serde::Serialize;

use crate::api;
use crate::api::client::{ApiClient, ApiClientError};

pub(super) fn run_status_command(args: &[String]) -> std::io::Result<i32> {
    let Some((scope, json)) = parse_status_args(args) else {
        return Ok(2);
    };

    match scope {
        StatusScope::Full => print_full_status(json),
        StatusScope::Server => print_server_status(json),
        StatusScope::Client => {
            print_client_status(json)?;
            Ok(0)
        }
        StatusScope::Help => {
            print_status_help();
            Ok(0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusScope {
    Full,
    Server,
    Client,
    Help,
}

fn parse_status_args(args: &[String]) -> Option<(StatusScope, bool)> {
    match args.first().map(|arg| arg.as_str()) {
        None => Some((StatusScope::Full, false)),
        Some("--json") if args.len() == 1 => Some((StatusScope::Full, true)),
        Some("server") => {
            parse_status_scope_args(args, StatusScope::Server, "hall status server [--json]")
        }
        Some("client") => {
            parse_status_scope_args(args, StatusScope::Client, "hall status client [--json]")
        }
        Some("help" | "--help" | "-h") => {
            if args.len() > 1 {
                print_status_help();
                return None;
            }
            Some((StatusScope::Help, false))
        }
        Some(_) => {
            print_status_help();
            None
        }
    }
}

fn parse_status_scope_args(
    args: &[String],
    scope: StatusScope,
    usage: &str,
) -> Option<(StatusScope, bool)> {
    match args.get(1).map(|arg| arg.as_str()) {
        None => Some((scope, false)),
        Some("--json") if args.len() == 2 => Some((scope, true)),
        _ => {
            eprintln!("usage: {usage}");
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerRuntimeStatus {
    Running {
        version: Option<String>,
        protocol: Option<u32>,
        capabilities: Option<crate::api::schema::ServerCapabilities>,
    },
    NotRunning,
}

impl ServerRuntimeStatus {
    /// hall: whether the running server is hall's (decision 45); `None` when
    /// none is running.
    fn hall_server(&self) -> Option<bool> {
        self.runtime_status().map(|status| status.is_hall_server())
    }

    fn runtime_status(&self) -> Option<api::RuntimeStatus> {
        match self {
            ServerRuntimeStatus::Running {
                version,
                protocol,
                capabilities,
            } => Some(api::RuntimeStatus {
                version: version.clone(),
                protocol: *protocol,
                capabilities: capabilities.clone(),
            }),
            ServerRuntimeStatus::NotRunning => None,
        }
    }
}

fn print_full_status(json: bool) -> std::io::Result<i32> {
    let server = read_server_runtime_status()?;

    if json {
        print_json(&FullStatusJson {
            client: client_status_json(),
            server: server_status_json(&server),
            update: update_status_json(&server),
        })?;
        return Ok(0);
    }

    println!("client:");
    println!("  version: {}", crate::build_info::version());
    println!(
        "  channel: {}",
        crate::config::Config::load().config.update.channel.as_str()
    );
    println!("  protocol: {}", crate::protocol::PROTOCOL_VERSION);
    println!(
        "  endpoint_protocol_generation: {}",
        crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION
    );
    println!();
    println!("server:");
    print_server_status_body(&server, "  ");
    println!();
    println!("update:");
    println!("  restart_needed: {}", restart_needed_label(&server));
    println!(
        "  server_binary_stale: {}",
        server_binary_stale_label(&server)
    );

    Ok(0)
}

fn print_server_status(json: bool) -> std::io::Result<i32> {
    let server = read_server_runtime_status()?;
    if json {
        print_json(&server_status_json(&server))?;
        return Ok(0);
    }
    print_server_status_body(&server, "");
    Ok(0)
}

fn print_client_status(json: bool) -> std::io::Result<()> {
    if json {
        print_json(&client_status_json())?;
        return Ok(());
    }

    println!("version: {}", crate::build_info::version());
    println!(
        "channel: {}",
        crate::config::Config::load().config.update.channel.as_str()
    );
    println!("protocol: {}", crate::protocol::PROTOCOL_VERSION);
    println!(
        "endpoint_protocol_generation: {}",
        crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION
    );
    println!("binary: {}", current_exe_label());
    Ok(())
}

fn print_server_status_body(server: &ServerRuntimeStatus, indent: &str) {
    match server {
        ServerRuntimeStatus::Running {
            version,
            protocol,
            capabilities,
        } => {
            println!("{indent}status: running");
            // hall: a server that is not hall's is named as such, with no
            // restart or stop guidance (decision 45).
            let hall_server = server.hall_server() == Some(true);
            println!(
                "{indent}hall_server: {}",
                if hall_server { "yes" } else { "no" }
            );
            if let (false, Some(status)) = (hall_server, server.runtime_status()) {
                for line in
                    crate::session::not_hall_server_message(&api::socket_path(), &status).lines()
                {
                    println!("{indent}  {line}");
                }
            }
            println!("{indent}version: {}", option_label(version.as_deref()));
            println!(
                "{indent}endpoint_compatible: {}",
                endpoint_compatibility_label(capabilities.as_ref())
            );
            println!("{indent}private_protocol: {}", protocol_label(*protocol));
            println!(
                "{indent}private_protocol_compatible: {}",
                compatibility_label(*protocol)
            );
            println!("{indent}socket: {}", api::socket_path().display());
        }
        ServerRuntimeStatus::NotRunning => {
            println!("{indent}status: not running");
            println!("{indent}socket: {}", api::socket_path().display());
        }
    }
}

fn read_server_runtime_status() -> std::io::Result<ServerRuntimeStatus> {
    // hall: the status request is bounded as the client launch's is, and a
    // listener that does not answer it gets the launch's message, not a hang
    // (decision 46).
    let client = ApiClient::local();
    match client.status_with_timeout(crate::server::autodetect::STATUS_REQUEST_TIMEOUT) {
        Ok(status) => Ok(ServerRuntimeStatus::Running {
            version: status.version,
            protocol: status.protocol,
            capabilities: status.capabilities,
        }),
        Err(ApiClientError::Io(err)) if super::server_not_running_error(&err) => {
            Ok(ServerRuntimeStatus::NotRunning)
        }
        Err(ApiClientError::Io(err))
            if matches!(
                err.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            tracing::warn!(err = %err, "server status check failed");
            Err(std::io::Error::other(
                crate::server::autodetect::unanswered_status_message(&client.socket_path()),
            ))
        }
        Err(err) => Err(api_client_error_to_io(err)),
    }
}

fn api_client_error_to_io(err: ApiClientError) -> std::io::Error {
    match err {
        ApiClientError::Io(err) => err,
        err => std::io::Error::other(err),
    }
}

fn option_label(value: Option<&str>) -> &str {
    value.unwrap_or("unknown")
}

fn protocol_label(protocol: Option<u32>) -> String {
    protocol
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn compatibility_label(protocol: Option<u32>) -> &'static str {
    match protocol {
        Some(protocol) if protocol == crate::protocol::PROTOCOL_VERSION => "yes",
        Some(_) => "no",
        None => "unknown",
    }
}

fn endpoint_compatibility_label(
    capabilities: Option<&crate::api::schema::ServerCapabilities>,
) -> &'static str {
    match capabilities.and_then(|value| value.endpoint_protocol_generation) {
        Some(generation)
            if generation == crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION =>
        {
            "yes"
        }
        Some(_) => "no",
        None => "unknown",
    }
}

/// hall: what the update lines say for a server that is not hall's.
const NOT_HALL_LABEL: &str = "n/a (not a hall server)";

fn restart_needed_label(server: &ServerRuntimeStatus) -> &'static str {
    if server.hall_server() == Some(false) {
        return NOT_HALL_LABEL;
    }
    match restart_needed_bool(server) {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

fn server_binary_stale_label(server: &ServerRuntimeStatus) -> &'static str {
    if server.hall_server() == Some(false) {
        return NOT_HALL_LABEL;
    }
    match server_binary_stale_bool(server) {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

#[derive(Serialize)]
struct FullStatusJson {
    client: ClientStatusJson,
    server: ServerStatusJson,
    update: UpdateStatusJson,
}

#[derive(Serialize)]
struct ClientStatusJson {
    version: String,
    channel: &'static str,
    protocol: u32,
    endpoint_protocol_generation: u32,
    endpoint_capabilities: Vec<&'static str>,
    binary: String,
    session: Option<String>,
}

#[derive(Serialize)]
struct ServerStatusJson {
    status: &'static str,
    running: bool,
    hall_server: Option<bool>,
    version: Option<String>,
    protocol: Option<u32>,
    capabilities: Option<ServerCapabilitiesJson>,
    compatible: Option<bool>,
    endpoint_compatible: Option<bool>,
    socket: String,
    session: Option<String>,
    restart_needed: Option<bool>,
    server_binary_stale: Option<bool>,
}

#[derive(Serialize)]
struct ServerCapabilitiesJson {
    live_handoff: bool,
    detached_server_daemon: bool,
    endpoint_protocol_generation: Option<u32>,
    surface_interest: bool,
    health_check: bool,
}

#[derive(Serialize)]
struct UpdateStatusJson {
    restart_needed: Option<bool>,
    server_binary_stale: Option<bool>,
}

fn client_status_json() -> ClientStatusJson {
    ClientStatusJson {
        version: crate::build_info::version(),
        channel: crate::config::Config::load().config.update.channel.as_str(),
        protocol: crate::protocol::PROTOCOL_VERSION,
        endpoint_protocol_generation: crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION,
        endpoint_capabilities: vec![
            crate::protocol::endpoint::SURFACE_INTEREST_CAPABILITY,
            crate::protocol::endpoint::PRESENTATION_EFFECTS_FENCE_CAPABILITY,
            crate::protocol::endpoint::HEALTH_CHECK_CAPABILITY,
        ],
        binary: current_exe_label(),
        session: crate::session::active_name(),
    }
}

fn server_status_json(server: &ServerRuntimeStatus) -> ServerStatusJson {
    match server {
        ServerRuntimeStatus::Running {
            version,
            protocol,
            capabilities,
        } => ServerStatusJson {
            status: "running",
            running: true,
            hall_server: server.hall_server(),
            version: version.clone(),
            protocol: *protocol,
            capabilities: capabilities
                .as_ref()
                .map(|capabilities| ServerCapabilitiesJson {
                    live_handoff: capabilities.live_handoff,
                    detached_server_daemon: capabilities.detached_server_daemon,
                    endpoint_protocol_generation: capabilities.endpoint_protocol_generation,
                    surface_interest: capabilities.surface_interest,
                    health_check: capabilities.health_check,
                }),
            compatible: protocol.map(|value| value == crate::protocol::PROTOCOL_VERSION),
            endpoint_compatible: capabilities.as_ref().and_then(|capabilities| {
                capabilities.endpoint_protocol_generation.map(|generation| {
                    generation == crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION
                })
            }),
            socket: api::socket_path().display().to_string(),
            session: crate::session::active_name(),
            restart_needed: restart_needed_bool(server),
            server_binary_stale: server_binary_stale_bool(server),
        },
        ServerRuntimeStatus::NotRunning => ServerStatusJson {
            status: "not_running",
            running: false,
            hall_server: None,
            version: None,
            protocol: None,
            capabilities: None,
            compatible: None,
            endpoint_compatible: None,
            socket: api::socket_path().display().to_string(),
            session: crate::session::active_name(),
            restart_needed: Some(false),
            server_binary_stale: Some(false),
        },
    }
}

fn update_status_json(server: &ServerRuntimeStatus) -> UpdateStatusJson {
    UpdateStatusJson {
        restart_needed: restart_needed_bool(server),
        server_binary_stale: server_binary_stale_bool(server),
    }
}

/// hall: `None` for a server that is not hall's: restarting it is not hall's
/// to suggest (decision 45).
fn restart_needed_bool(server: &ServerRuntimeStatus) -> Option<bool> {
    if server.hall_server() == Some(false) {
        return None;
    }
    match server {
        ServerRuntimeStatus::Running { capabilities, .. } => Some(
            capabilities
                .as_ref()
                .and_then(|value| value.endpoint_protocol_generation)
                != Some(crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION),
        ),
        ServerRuntimeStatus::NotRunning => Some(false),
    }
}

fn server_binary_stale_bool(server: &ServerRuntimeStatus) -> Option<bool> {
    if server.hall_server() == Some(false) {
        return None;
    }
    match server {
        ServerRuntimeStatus::Running { version, .. } => version
            .as_deref()
            .map(|version| version != crate::build_info::version()),
        ServerRuntimeStatus::NotRunning => Some(false),
    }
}

fn print_json(value: &impl Serialize) -> std::io::Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn current_exe_label() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|err| format!("unknown ({err})"))
}

fn print_status_help() {
    eprintln!("hall status commands:");
    eprintln!("  hall status [--json]         show local client and running server status");
    eprintln!("  hall status server [--json]  show running server status");
    eprintln!("  hall status client [--json]  show local client binary status");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running_server(
        version: Option<&str>,
        endpoint_generation: Option<u32>,
    ) -> ServerRuntimeStatus {
        ServerRuntimeStatus::Running {
            version: version.map(str::to_owned),
            protocol: Some(crate::protocol::PROTOCOL_VERSION),
            capabilities: Some(crate::api::schema::ServerCapabilities {
                live_handoff: true,
                detached_server_daemon: true,
                endpoint_protocol_generation: endpoint_generation,
                surface_interest: true,
                health_check: true,
            }),
        }
    }

    #[test]
    fn stale_compatible_server_does_not_require_restart() {
        let server = running_server(
            Some("0.0.0-old"),
            Some(crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION),
        );

        assert_eq!(restart_needed_bool(&server), Some(false));
        assert_eq!(server_binary_stale_bool(&server), Some(true));
    }

    #[test]
    fn hall_server_with_another_endpoint_generation_requires_restart() {
        let server = running_server(
            Some(crate::build_info::version().as_str()),
            Some(crate::api::HALL_NUMBER_FLOOR),
        );

        assert_eq!(server.hall_server(), Some(true));
        assert_eq!(restart_needed_bool(&server), Some(true));
        assert_eq!(server_binary_stale_bool(&server), Some(false));
    }

    /// hall: a server with no endpoint generation, or herdr's numbers, is not
    /// confirmed as hall's, so no restart is suggested for it (decisions 45, 46).
    #[test]
    fn server_that_is_not_hall_gets_no_restart_guidance() {
        let herdr = ServerRuntimeStatus::Running {
            version: Some("0.9.0".to_owned()),
            protocol: Some(22),
            capabilities: Some(crate::api::schema::ServerCapabilities {
                live_handoff: true,
                detached_server_daemon: true,
                endpoint_protocol_generation: Some(1),
                surface_interest: true,
                health_check: true,
            }),
        };
        let no_generation = running_server(Some(crate::build_info::version().as_str()), None);
        for server in [herdr, no_generation] {
            assert_eq!(server.hall_server(), Some(false));
            assert_eq!(restart_needed_bool(&server), None);
            assert_eq!(server_binary_stale_bool(&server), None);
            assert_eq!(restart_needed_label(&server), NOT_HALL_LABEL);
            assert_eq!(server_status_json(&server).hall_server, Some(false));
        }
        assert_eq!(ServerRuntimeStatus::NotRunning.hall_server(), None);
        assert_eq!(
            restart_needed_bool(&ServerRuntimeStatus::NotRunning),
            Some(false)
        );
    }
}
