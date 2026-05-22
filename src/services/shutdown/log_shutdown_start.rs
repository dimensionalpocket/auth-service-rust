use tracing::info;

pub struct LogShutdownStartService;

impl LogShutdownStartService {
  pub fn run(signal_name: &str) {
    info!("Starting graceful shutdown due to {} signal", signal_name);
    info!("Waiting for existing connections to complete...");
  }
}
