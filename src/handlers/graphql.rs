use crate::graphql::schema::AppSchema;
use crate::middleware::session::SessionContext;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
  extract::{Extension, State},
  http::StatusCode,
  response::{Html, IntoResponse, Response},
};
use tracing::instrument;

// Note: resolvers should use async-graphql's `ctx.insert_http_header` / `ctx.append_http_header`
// APIs to write HTTP headers directly into the GraphQL response. The handler relies on
// `GraphQLResponse::into_response()` to include those headers in the final HTTP response.

/// GraphQL handler for both GET and POST requests
#[instrument(skip(schema, req))]
pub async fn graphql_handler(
  State(schema): State<AppSchema>,
  Extension(session): Extension<SessionContext>,
  req: GraphQLRequest,
) -> GraphQLResponse {
  // Inject session into GraphQL request context (3 lines of logic!)
  let request = req.into_inner().data(session);

  // Execute and return response
  schema.execute(request).await.into()
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
