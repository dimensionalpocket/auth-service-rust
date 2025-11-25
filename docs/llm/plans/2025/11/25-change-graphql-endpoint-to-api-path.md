# Plan: Change GraphQL Endpoint from /graphql to /api/graphql

## Date
2025-11-25@14:33

## Overview
Change the GraphQL API endpoint from `/graphql` to `/api/graphql` where the `/api` prefix comes from the `api_path` property in the resolved server config. REST endpoints remain unchanged.

## Implementation Details

### Files to Modify

#### 1. `src/dps_auth_api.rs`
**Location**: `build_router` method (lines 183-213)
**Changes**: 
- Update the GraphQL route from `/graphql` to use the dynamic `api_path` config
- Change line 188 from `.route("/graphql",` to `.route(&format!("{}/graphql", self.config.api_path),`

**Code Sample**:
```rust
.route(
  &format!("{}/graphql", self.config.api_path),
  get({
    let development_mode = self.config.development_mode;
    move || crate::handlers::graphql::graphql_get_handler(development_mode)
  })
  .post({
    let config = self.config.clone();
    move |state, request| {
      crate::handlers::graphql::graphql_post_handler(state, request, config.clone())
    }
  })
  .layer(from_fn(
    crate::middleware::session::create_session_middleware(self.config.session_secret.clone()),
  )),
)
```

#### 2. `README.md`
**Location**: API Endpoints section (lines 18-24)
**Changes**: Update GraphQL endpoint documentation
- Change line 22 from `- GET /graphql - GraphQL playground (development mode only)` to `- GET {api_path}/graphql - GraphQL playground (development mode only)`
- Change line 23 from `- POST /graphql - GraphQL API` to `- POST {api_path}/graphql - GraphQL API`

**Code Sample**:
```markdown
## API Endpoints

- `GET /` - Root endpoint
- `GET /health` - Health check
- `GET {api_path}/graphql` - GraphQL playground (development mode only)
- `POST {api_path}/graphql` - GraphQL API
```

#### 3. Test Updates in `src/dps_auth_api.rs`
**Location**: Test methods that test GraphQL endpoint (lines 389-421)
**Changes**: Update test URIs to use the new endpoint path
- Update line 403 from `.uri("/graphql")` to `.uri(&format!("{}/graphql", server.config.api_path))`

**Code Sample**:
```rust
let response = app
  .oneshot(
    Request::builder()
      .method("POST")
      .uri(&format!("{}/graphql", server.config.api_path))
      .header("content-type", "application/json")
      .body(Body::from(query))
      .unwrap(),
  )
  .await
  .unwrap();
```

### Implementation Steps

1. **Update Router Configuration**: Modify the `build_router` method to use dynamic `api_path` for GraphQL routes
2. **Update Documentation**: Update README.md to reflect the new endpoint structure
3. **Update Tests**: Modify test cases to use the new dynamic endpoint path
4. **Verify Configuration**: Ensure `api_path` is properly configured in `DpsAuthApiConfig` (already exists as line 19)

### Technical Notes

- The `api_path` property already exists in `DpsAuthApiConfig` and is set from `dps_config.get_api_path()` with a leading slash
- The `format!("{}/graphql", self.config.api_path)` will result in `/api/graphql` by default
- REST endpoints (`/`, `/health`) remain unchanged as specified
- Session middleware and all other GraphQL functionality remains the same
- The change is backward compatible only if clients update their endpoint URLs

### Testing Requirements

- Verify GraphQL endpoint is accessible at `/api/graphql`
- Verify GraphQL playground works in development mode at `/api/graphql`
- Verify REST endpoints still work at `/` and `/health`
- Run existing test suite to ensure no regressions
- Test with different `api_path` configurations if available

### TODO
- Update README.md mutations/queries table if needed (no changes required for endpoint documentation)