use tokio::signal;
use tracing::info;

pub struct WaitForShutdownSignalService;

impl WaitForShutdownSignalService {
  pub async fn run() -> &'static str {
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
}
