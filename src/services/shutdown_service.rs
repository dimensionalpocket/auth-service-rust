use tokio::signal;
use tracing::info;

pub struct ShutdownService;

impl ShutdownService {
  /// Wait for shutdown signals (SIGTERM or SIGINT)
  /// Returns the name of the signal that was received
  pub async fn wait_for_shutdown_signal() -> &'static str {
    let ctrl_c = async {
      signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
      signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to install signal handler")
        .recv()
        .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
      _ = ctrl_c => {
        info!("Received SIGINT signal, initiating graceful shutdown");
        "SIGINT"
      }
      _ = terminate => {
        info!("Received SIGTERM signal, initiating graceful shutdown");
        "SIGTERM"
      }
    }
  }

  /// Log the shutdown initiation - the actual graceful shutdown is handled by axum
  pub fn log_shutdown_start(signal_name: &str) {
    info!("Starting graceful shutdown due to {} signal", signal_name);
    info!("Waiting for existing connections to complete...");
  }
}
