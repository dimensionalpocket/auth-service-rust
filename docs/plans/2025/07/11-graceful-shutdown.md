# Graceful Shutdown Implementation Plan

**Date**: 2025-07-11@20:14  
**Task**: Implement graceful shutdown for the API with SIGTERM and SIGINT signal handling

## Overview

This plan outlines the implementation of graceful shutdown functionality for the dp-auth-service API. The service should properly handle SIGTERM and SIGINT signals, log which signal was received, and gracefully shut down all active connections and resources.

## Current State Analysis

- The server is currently using `axum::serve()` with a `TcpListener`
- No signal handling is currently implemented
- The server runs indefinitely with `axum::serve(listener, app).await.unwrap()`
- Tokio runtime provides signal handling capabilities via `tokio::signal`

## Implementation Strategy

### 1. Dependencies
- **tokio::signal**: Already available through tokio with "full" features
- No additional dependencies required

### 2. Architecture Changes

#### Main Function Modifications
- Replace the simple `axum::serve().await` with a more sophisticated setup
- Implement signal handling using `tokio::signal::ctrl_c()` for SIGINT and `tokio::signal::unix::signal()` for SIGTERM
- Use `tokio::select!` to race between server operation and signal reception
- Add graceful shutdown with configurable timeout

#### Signal Handling Service
Create a new service `ShutdownService` in `src/services/shutdown_service.rs`:
- Handle signal registration and monitoring
- Provide shutdown coordination
- Log which signal was received
- Manage shutdown timeout

## Files to be Created/Modified

### New Files

#### `src/services/shutdown_service.rs`
```rust
use tokio::signal;
use tracing::{info, warn};

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
```

### Modified Files

#### `src/main.rs`
```rust
// Add imports
use tokio::signal;
use dp_auth_service::services::shutdown_service::ShutdownService;

// Replace the server startup section (lines 56-59) with:
let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
println!("Server running on http://{bind_address}");

// Create the server with graceful shutdown
let server = axum::serve(listener, app).with_graceful_shutdown(async {
  let signal_name = ShutdownService::wait_for_shutdown_signal().await;
  ShutdownService::log_shutdown_start(signal_name);
  // The actual graceful shutdown is handled by axum's with_graceful_shutdown
  // This future completes when the signal is received, triggering axum's shutdown
});

// Run the server
if let Err(e) = server.await {
  tracing::error!("Server error: {}", e);
}
```

#### `src/services/mod.rs`
```rust
// Add new module
pub mod shutdown_service;
```

#### `src/lib.rs` 
No changes needed - services module already exported.

## Implementation Details

### Signal Handling
- **SIGINT (Ctrl+C)**: Handled via `tokio::signal::ctrl_c()`
- **SIGTERM**: Handled via `tokio::signal::unix::signal()` on Unix systems
- **Cross-platform**: Windows support through conditional compilation

### Graceful Shutdown Process
1. Signal received and logged with signal type
2. `axum::serve()` graceful shutdown initiated automatically
3. Server stops accepting new connections
4. Existing connections are allowed to complete naturally
5. Server shuts down cleanly when all connections are closed
6. Application exits cleanly

**Note**: The actual graceful shutdown logic is handled by axum's `with_graceful_shutdown()`. When the future we provide completes (i.e., when a signal is received), axum automatically:
- Stops accepting new connections
- Waits for existing connections to finish their current requests
- Shuts down the server cleanly

### Logging
- Log when signal is received with signal type
- Log start of graceful shutdown process
- Log completion of graceful shutdown
- Use structured logging with tracing

### Configuration
- No timeout configuration needed - axum handles the graceful shutdown automatically
- Future enhancements could add timeout configuration if needed

## Testing Strategy

### Unit Tests
- Test `ShutdownService::wait_for_shutdown_signal()` with mock signals
- Test graceful shutdown timeout behavior

### Integration Tests
- Test that server responds to SIGTERM correctly
- Test that server responds to SIGINT correctly
- Verify that active connections are handled properly during shutdown

### Manual Testing
- Start server and send SIGTERM: `kill -TERM <pid>`
- Start server and send SIGINT: `Ctrl+C`
- Verify console output shows correct signal name
- Test with active connections during shutdown

## Benefits

1. **Proper Resource Cleanup**: Database connections and other resources are properly closed
2. **Zero Downtime Deployments**: Allows for graceful rolling updates
3. **Better Observability**: Clear logging of shutdown events
4. **Production Ready**: Follows industry standards for service lifecycle management
5. **Container Friendly**: Properly handles container orchestration signals

## Considerations

- The current implementation will be basic but extensible
- Future enhancements could include:
  - Configurable timeout via environment variables
  - More sophisticated connection draining
  - Health check endpoint updates during shutdown
  - Metrics collection for shutdown events

## Timeline

This implementation should take approximately 2-3 hours:
- 1 hour: Core implementation
- 1 hour: Testing and refinement
- 30 minutes: Documentation and cleanup