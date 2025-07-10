use axum::{extract::Request, middleware::Next, response::Response};
use tracing::{info_span, Instrument};
use uuid::Uuid;

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
  let request_id = Uuid::new_v4().to_string();
  request
    .extensions_mut()
    .insert(RequestId(request_id.clone()));

  // Create a span with request_id that will be inherited by all operations in this request
  let span = info_span!("request", request_id = %request_id);

  next.run(request).instrument(span).await
}

#[derive(Clone)]
pub struct RequestId(pub String);
