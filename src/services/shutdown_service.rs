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

#[cfg(test)]
mod tests {
  use super::*;
  use tracing_test::traced_test;

  #[test]
  #[traced_test]
  fn test_log_shutdown_start_logs_correct_message() {
    // Test that log_shutdown_start logs the expected messages
    ShutdownService::log_shutdown_start("SIGTERM");

    // Verify the log messages were written
    assert!(logs_contain(
      "Starting graceful shutdown due to SIGTERM signal"
    ));
    assert!(logs_contain(
      "Waiting for existing connections to complete..."
    ));
  }

  #[test]
  #[traced_test]
  fn test_log_shutdown_start_with_sigint() {
    // Test with SIGINT signal
    ShutdownService::log_shutdown_start("SIGINT");

    // Verify the log messages were written
    assert!(logs_contain(
      "Starting graceful shutdown due to SIGINT signal"
    ));
    assert!(logs_contain(
      "Waiting for existing connections to complete..."
    ));
  }

  #[test]
  #[traced_test]
  fn test_log_shutdown_start_with_custom_signal() {
    // Test with a custom signal name
    ShutdownService::log_shutdown_start("TEST_SIGNAL");

    // Verify the log messages were written
    assert!(logs_contain(
      "Starting graceful shutdown due to TEST_SIGNAL signal"
    ));
    assert!(logs_contain(
      "Waiting for existing connections to complete..."
    ));
  }
}
