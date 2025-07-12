use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

pub async fn rest_logging_middleware(request: Request, next: Next) -> Response {
  let path = request.uri().path().to_string();
  let query = request.uri().query().unwrap_or("").to_string();
  let method = request.method().clone();

  // Calculate content length if present
  let content_length = if let Some(content_length_header) = request.headers().get("content-length")
  {
    content_length_header
      .to_str()
      .unwrap_or("0")
      .parse::<usize>()
      .unwrap_or(0)
  } else {
    0
  };

  let start = Instant::now();
  let response = next.run(request).await;
  let duration = start.elapsed();
  let status = response.status();

  // Log for REST endpoints (existing routes + 404s)
  let should_log = path == "/" || path == "/health" || status == StatusCode::NOT_FOUND;

  if should_log {
    let query_part = if query.is_empty() {
      String::new()
    } else {
      format!("?{query}")
    };

    info!(
      method = %method,
      path = %format!("{}{}", path, query_part),
      status = %status,
      duration_ms = duration.as_millis(),
      content_length = content_length,
      "REST Request"
    );
  }

  response
}
