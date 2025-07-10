use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

pub async fn rest_logging_middleware(request: Request, next: Next) -> Response {
  // Check if this is a REST endpoint first
  let path = request.uri().path();
  let should_log = path == "/" || path == "/health";

  let start = if should_log {
    Some(Instant::now())
  } else {
    None
  };
  let method = if should_log {
    Some(request.method().clone())
  } else {
    None
  };
  let path_string = if should_log {
    Some(path.to_string())
  } else {
    None
  };

  let response = next.run(request).await;

  // Only log for REST endpoints
  if should_log {
    let duration = start.unwrap().elapsed();
    let status = response.status();

    info!(
      method = %method.unwrap(),
      path = %path_string.unwrap(),
      status = %status,
      duration_ms = duration.as_millis(),
      "REST Request"
    );
  }

  response
}
