use std::io;
use std::path::Path;
use std::time::Duration;

use crate::api::schema::{Method, Request, ResponseResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub version: Option<String>,
    pub protocol: Option<u32>,
    pub capabilities: Option<crate::api::schema::ServerCapabilities>,
}

/// hall: hall's protocol numbers and endpoint generations are all at or above
/// this floor (decision 33); herdr's are below it.
pub const HALL_NUMBER_FLOOR: u32 = 1000;

impl RuntimeStatus {
    /// hall: the server is hall's only when both its protocol and its endpoint
    /// generation are in hall's range (decision 45). A missing number fails.
    pub fn is_hall_server(&self) -> bool {
        let generation = self.endpoint_protocol_generation();
        self.protocol
            .is_some_and(|protocol| protocol >= HALL_NUMBER_FLOOR)
            && generation.is_some_and(|generation| generation >= HALL_NUMBER_FLOOR)
    }

    pub fn endpoint_protocol_generation(&self) -> Option<u32> {
        self.capabilities
            .as_ref()
            .and_then(|capabilities| capabilities.endpoint_protocol_generation)
    }
}

pub fn read_runtime_status_at(
    socket_path: &Path,
    timeout: Duration,
) -> io::Result<Option<RuntimeStatus>> {
    if !socket_path.exists() {
        return Ok(None);
    }

    let client = crate::api::client::ApiClient::for_target(
        crate::api::client::ConnectionTarget::SocketPath(socket_path.to_path_buf()),
    );
    let request = Request {
        id: "runtime:status".into(),
        method: Method::Ping(crate::api::schema::PingParams::default()),
    };
    let response = client
        .request_value_with_timeout(&request, timeout)
        .and_then(crate::api::client::parse_response_value);
    let response = match response {
        Ok(response) => response,
        Err(crate::api::client::ApiClientError::Io(err))
            if matches!(
                err.kind(),
                io::ErrorKind::ConnectionRefused
                    | io::ErrorKind::NotFound
                    | io::ErrorKind::TimedOut
            ) =>
        {
            return Ok(None);
        }
        Err(err) => return Err(io::Error::other(err)),
    };
    match response.result {
        ResponseResult::Pong {
            version,
            protocol,
            capabilities,
        } => Ok(Some(RuntimeStatus {
            version: Some(version),
            protocol: Some(protocol),
            capabilities,
        })),
        result => Err(io::Error::other(format!(
            "server status request returned unexpected result: {result:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(protocol: Option<u32>, generation: Option<u32>) -> RuntimeStatus {
        RuntimeStatus {
            version: Some("0.9.0".into()),
            protocol,
            capabilities: Some(crate::api::schema::ServerCapabilities {
                live_handoff: true,
                detached_server_daemon: false,
                endpoint_protocol_generation: generation,
                surface_interest: false,
                health_check: false,
            }),
        }
    }

    #[test]
    fn hall_server_needs_both_numbers_in_hall_range() {
        assert!(status(Some(1022), Some(1001)).is_hall_server());
        assert!(status(Some(1000), Some(1000)).is_hall_server());
        assert!(!status(Some(22), Some(1)).is_hall_server());
        assert!(!status(Some(1022), Some(1)).is_hall_server());
        assert!(!status(Some(22), Some(1001)).is_hall_server());
        assert!(!status(Some(999), Some(1001)).is_hall_server());
        assert!(!status(None, Some(1001)).is_hall_server());
        assert!(!status(Some(1022), None).is_hall_server());
        let no_capabilities = RuntimeStatus {
            capabilities: None,
            ..status(Some(1022), Some(1001))
        };
        assert!(!no_capabilities.is_hall_server());
    }
}
