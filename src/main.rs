mod routes;

use std::io::{self, IsTerminal};

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        // Colors only on a terminal: Kubernetes and CI logs would show escape codes.
        .with_ansi(io::stdout().is_terminal())
        .init();

    let (host, port) = listen_address(std::env::var("HOST").ok(), std::env::var("PORT").ok())?;
    let listener = TcpListener::bind((host.as_str(), port)).await?;
    tracing::info!(address = %listener.local_addr()?, "server starting");

    axum::serve(listener, routes::router())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("server stopped");
    Ok(())
}

/// Listens on all interfaces at port 8080 unless HOST or PORT say otherwise.
fn listen_address(host: Option<String>, port: Option<String>) -> io::Result<(String, u16)> {
    let host = host
        .filter(|host| !host.is_empty())
        .unwrap_or_else(|| "0.0.0.0".to_owned());
    let port = match port.filter(|port| !port.is_empty()) {
        Some(port) => port.parse().map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("PORT {port}: {error}"))
        })?,
        None => 8080,
    };
    Ok((host, port))
}

/// Resolves on SIGTERM, which Kubernetes sends before stopping a pod, or on Ctrl+C.
/// In-flight requests are then completed before the server stops.
async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "listen for Ctrl+C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}

#[cfg(test)]
mod tests {
    use super::listen_address;

    #[test]
    fn listens_on_port_8080_by_default() {
        let address = listen_address(None, Some(String::new())).unwrap();
        assert_eq!(address, ("0.0.0.0".to_owned(), 8080));
    }

    #[test]
    fn reads_host_and_port() {
        let address = listen_address(Some("::".into()), Some("3000".into())).unwrap();
        assert_eq!(address, ("::".to_owned(), 3000));
    }

    #[test]
    fn rejects_an_invalid_port() {
        assert!(listen_address(None, Some("http".into())).is_err());
    }
}
