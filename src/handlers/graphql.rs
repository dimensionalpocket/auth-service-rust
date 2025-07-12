use crate::graphql::schema::AppSchema;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
  extract::State,
  http::StatusCode,
  response::{Html, IntoResponse, Response},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::env;
use std::time::Instant;
use tracing::{error, info, instrument};

static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

/// GraphQL POST handler for actual queries
#[instrument(skip(schema, req))]
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  req: GraphQLRequest,
) -> impl IntoResponse {
  let start = Instant::now();

  let request = req.into_inner();

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

  let graphql_response: GraphQLResponse = response.into();
  graphql_response
}

/// GraphQL GET handler for playground (development only)
pub async fn graphql_get_handler() -> Response {
  if env::var("APP_ENV").unwrap_or_default() == "development" {
    Html(playground_html()).into_response()
  } else {
    (
      StatusCode::METHOD_NOT_ALLOWED,
      "Method not allowed",
    )
      .into_response()
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
