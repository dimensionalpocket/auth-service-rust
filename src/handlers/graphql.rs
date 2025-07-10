use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
  extract::State,
  response::{Html, IntoResponse, Response},
  http::StatusCode,
};
use crate::graphql::schema::AppSchema;
use std::env;

/// GraphQL POST handler for actual queries
pub async fn graphql_post_handler(
  State(schema): State<AppSchema>,
  req: GraphQLRequest,
) -> impl IntoResponse {
  let response: GraphQLResponse = schema.execute(req.into_inner()).await.into();
  response
}

/// GraphQL GET handler for playground (development only)
pub async fn graphql_get_handler() -> Response {
  if env::var("APP_ENV").unwrap_or_default() == "development" {
    Html(playground_html()).into_response()
  } else {
    (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed - POST only").into_response()
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