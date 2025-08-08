use crate::graphql::schema::AppSchema;
use crate::middleware::session::SessionContext;
use async_graphql_axum::GraphQLResponse;
use axum::{
  extract::{Request, State},
  http::{HeaderMap, StatusCode},
  response::{Html, IntoResponse, Response},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{error, info, instrument};

static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

/// Container for response headers that can be set by GraphQL resolvers
pub struct ResponseHeaders {
  pub headers: Arc<Mutex<HeaderMap>>,
}

/// Set session cookie in response headers
pub fn set_session_cookie(
  response_headers: &Arc<Mutex<HeaderMap>>, 
  token: &str,
  cookie_domain: &str,
  insecure_cookie: bool
) {
  use crate::middleware::session::SESSION_COOKIE_NAME;
  let secure_flag = if insecure_cookie { "" } else { "; Secure" };

  let cookie_value = format!(
    "{}={}; Domain={}; Path=/; HttpOnly; SameSite=Strict{}; Max-Age={}",
    SESSION_COOKIE_NAME,
    token,
    cookie_domain,
    secure_flag,
    3 * 24 * 60 * 60 // 3 days in seconds
  );

  if let Ok(mut headers) = response_headers.lock() {
    if let Ok(header_value) = cookie_value.parse() {
      headers.insert("set-cookie", header_value);
    }
  }
}

/// GraphQL POST handler for actual queries
#[instrument(skip(schema, http_req))]
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  http_req: Request,
  cookie_domain: String,
  insecure_cookie: bool,
  development_mode: bool,
  session_secret: Vec<u8>,
) -> impl IntoResponse {
  let start = Instant::now();

  // Extract session context from HTTP request extensions (set by middleware)
  let session_context = http_req
    .extensions()
    .get::<SessionContext>()
    .cloned()
    .unwrap_or_else(|| SessionContext::new(None));

  // Create response headers container for resolvers to use
  let response_headers = ResponseHeaders {
    headers: Arc::new(Mutex::new(HeaderMap::new())),
  };

  // Parse GraphQL request from HTTP request
  let graphql_request = match parse_graphql_request(http_req).await {
    Ok(req) => req,
    Err(response) => return response,
  };

  // Add session context, response headers, and config to GraphQL request data
  let mut request = graphql_request;
  request = request.data(session_context);
  request = request.data(response_headers.headers.clone());
  request = request.data(cookie_domain);
  request = request.data(insecure_cookie);
  request = request.data(session_secret);

  // Extract operation name from the request
  let operation_name = request
    .operation_name
    .clone()
    .unwrap_or_else(|| "Anonymous".to_string());
  let query = if operation_name == "Anonymous" {
    Some(
      WHITESPACE_REGEX
        .replace_all(&request.query, " ")
        .to_string(),
    )
  } else {
    None
  };

  let response = schema.execute(request).await;
  let duration = start.elapsed();

  // Only log non-introspection queries
  if operation_name != "IntrospectionQuery" {
    if response.is_ok() {
      match &query {
        Some(q) => info!(
          operation_name = %operation_name,
          query = %q,
          duration_ms = duration.as_millis(),
          "GraphQL Request"
        ),
        None => info!(
          operation_name = %operation_name,
          duration_ms = duration.as_millis(),
          "GraphQL Request"
        ),
      }
    } else {
      match &query {
        Some(q) => error!(
          operation_name = %operation_name,
          query = %q,
          duration_ms = duration.as_millis(),
          "GraphQL Request Error"
        ),
        None => error!(
          operation_name = %operation_name,
          duration_ms = duration.as_millis(),
          "GraphQL Request Error"
        ),
      }
    }
  }

  // Convert GraphQL response and apply any headers set by resolvers
  let mut http_response = {
    let graphql_response: GraphQLResponse = response.into();
    graphql_response.into_response()
  };

  // Apply any headers that were set by resolvers
  if let Ok(resolver_headers) = response_headers.headers.lock() {
    let response_headers_mut = http_response.headers_mut();
    for (name, value) in resolver_headers.iter() {
      response_headers_mut.insert(name, value.clone());
    }
  }

  http_response
}

/// Parse GraphQL request from HTTP request
async fn parse_graphql_request(req: Request) -> Result<async_graphql::Request, Response> {
  use axum::body::to_bytes;

  // Extract headers and body
  let (parts, body) = req.into_parts();
  let headers = &parts.headers;

  // Read the body
  let body_bytes = match to_bytes(body, usize::MAX).await {
    Ok(bytes) => bytes,
    Err(_) => {
      return Err((StatusCode::BAD_REQUEST, "Failed to read request body").into_response());
    }
  };

  // Parse based on content type
  let content_type = headers
    .get("content-type")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");

  if content_type.contains("application/json") {
    // Parse JSON GraphQL request
    match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
      Ok(json) => {
        let query = json.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let operation_name = json.get("operationName").and_then(|v| v.as_str());
        let variables = json
          .get("variables")
          .cloned()
          .unwrap_or(serde_json::Value::Null);

        let mut request = async_graphql::Request::new(query);
        if let Some(name) = operation_name {
          request = request.operation_name(name);
        }
        if !variables.is_null() {
          if let Ok(vars) = serde_json::from_value(variables) {
            request = request.variables(vars);
          }
        }

        Ok(request)
      }
      Err(_) => Err((StatusCode::BAD_REQUEST, "Invalid JSON").into_response()),
    }
  } else {
    Err((StatusCode::BAD_REQUEST, "Unsupported content type").into_response())
  }
}

/// GraphQL GET handler for playground (development only)
pub async fn graphql_get_handler(development_mode: bool) -> Response {
  if development_mode {
    Html(playground_html()).into_response()
  } else {
    (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed").into_response()
  }
}

fn playground_html() -> String {
  r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset=utf-8/>
  <meta name="viewport" content="user-scalable=no, initial-scale=1.0, minimum-scale=1.0, maximum-scale=1.0, minimal-ui">
  <title>GraphQL Playground</title>
  <link rel="stylesheet" href="//cdn.jsdelivr.net/npm/graphql-playground-react/build/static/css/index.css" />
  <link rel="shortcut icon" href="//cdn.jsdelivr.net/npm/graphql-playground-react/build/favicon.png" />
  <script src="//cdn.jsdelivr.net/npm/graphql-playground-react/build/static/js/middleware.js"></script>
</head>
<body>
  <div id="root">
    <style>
      body {
        background-color: rgb(23, 42, 58);
        font-family: Open Sans, sans-serif;
        height: 90vh;
      }
      #root {
        height: 100%;
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
      }
      .loading {
        font-size: 32px;
        font-weight: 200;
        color: rgba(255, 255, 255, .6);
        margin-left: 20px;
      }
      img {
        width: 78px;
        height: 78px;
      }
      .title {
        font-weight: 400;
      }
    </style>
    <img src="//cdn.jsdelivr.net/npm/graphql-playground-react/build/logo.png" alt="">
    <div class="loading"> Loading
      <span class="title">GraphQL Playground</span>
    </div>
  </div>
  <script>window.addEventListener('load', function (event) {
      GraphQLPlayground.init(document.getElementById('root'), {
        endpoint: '/graphql'
      })
    })</script>
</body>
</html>
"#.to_string()
}
