use crate::graphql::schema::AppSchema;
use crate::middleware::session::SessionContext;
use async_graphql_axum::GraphQLResponse;
use axum::{
  extract::{Request, State},
  http::StatusCode,
  response::{Html, IntoResponse, Response},
};
use tracing::instrument;

// Note: resolvers should use async-graphql's `ctx.insert_http_header` / `ctx.append_http_header`
// APIs to write HTTP headers directly into the GraphQL response. The handler relies on
// `GraphQLResponse::into_response()` to include those headers in the final HTTP response.

/// GraphQL POST handler for actual queries
#[instrument(skip(schema, http_req))]
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  http_req: Request,
) -> impl IntoResponse {
  // Extract session context from HTTP request extensions (set by middleware)
  let session_context = http_req
    .extensions()
    .get::<SessionContext>()
    .cloned()
    .unwrap_or_else(|| SessionContext::new(None));

  // Parse GraphQL request from HTTP request
  let graphql_request = match parse_graphql_request(http_req).await {
    Ok(req) => req,
    Err(response) => return response,
  };

  // Add session context to GraphQL request data (config is already in schema data)
  let request = graphql_request.data(session_context);

  // Execute GraphQL request with built-in tracing from Tracing extension
  let response = schema.execute(request).await;

  // Convert GraphQL response (resolvers may have inserted headers via async-graphql APIs)
  let graphql_response: GraphQLResponse = response.into();
  graphql_response.into_response()
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

/// GraphQL GET handler - only allows POST operations
pub async fn graphql_get_handler() -> Response {
  (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed").into_response()
}

/// Playground handler for development mode only
pub async fn playground_handler(development_mode: bool) -> Response {
  if development_mode {
    Html(playground_html()).into_response()
  } else {
    (StatusCode::NOT_FOUND, "Not found").into_response()
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
