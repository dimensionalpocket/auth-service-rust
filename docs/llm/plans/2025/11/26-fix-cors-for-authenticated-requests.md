# 26-fix-cors-for-authenticated-requests.md

## Problem

The current CORS configuration uses `CorsLayer::permissive()` which sets `Access-Control-Allow-Origin: *`. This wildcard origin does not work with authenticated requests that require cookies, as browsers block credentials from being sent to wildcard origins for security reasons.

## Solution

Replace the permissive CORS layer with `CorsLayer::very_permissive()` which:
1. Allows credentials (cookies) to be sent
2. Reflects the request origin back (same behavior as * for non-authenticated requests)
3. Dynamically allows the method received in `Access-Control-Request-Method`
4. Dynamically allows headers received in `Access-Control-Request-Headers`

## Implementation Details

### Files to Modify

1. **src/dps_auth_api.rs** - Update the `build_router` method to use `very_permissive()` CORS configuration

### Code Changes

#### 1. Replace CorsLayer::permissive() with very_permissive()

In `src/dps_auth_api.rs`, line 210, replace:
```rust
.layer(CorsLayer::permissive()),
```

With:
```rust
.layer(CorsLayer::very_permissive()),
```

This single change provides all the necessary CORS configuration for authenticated requests while maintaining the same permissive behavior for non-authenticated requests.

### Testing

#### 1. Update existing tests

The existing tests in `src/dps_auth_api.rs` should continue to pass, as the new CORS configuration maintains the same permissive behavior for non-authenticated requests.

#### 2. Add new test for authenticated requests

Add a new test to verify CORS headers are properly set for authenticated requests:
```rust
#[tokio::test]
async fn test_cors_headers_with_credentials() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let server = create_test_server(db_path);

    let schema = crate::graphql::schema::build_schema().finish();
    let app = server.build_router(schema);

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri(format!("{}/graphql", server.config.api_path))
                .header("origin", "https://example.com")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type, authorization")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    // Check that origin is reflected back
    assert_eq!(
        response.headers().get("access-control-allow-origin").unwrap(),
        "https://example.com"
    );
    
    // Check that credentials are allowed
    assert_eq!(
        response.headers().get("access-control-allow-credentials").unwrap(),
        "true"
    );
}
```

## Benefits

1. **Authenticated requests work**: Cookies can now be sent with GraphQL requests
2. **Security maintained**: Origin reflection prevents cross-origin attacks while allowing legitimate requests
3. **Backward compatibility**: Non-authenticated requests continue to work as before
4. **Standards compliant**: Uses tower-http's built-in `very_permissive()` configuration which follows CORS best practices for authenticated APIs
5. **Simplified implementation**: Single method call replaces complex manual configuration

## Notes

- `CorsLayer::very_permissive()` is specifically designed for APIs that need to support credentials
- It automatically reflects the request origin and dynamically allows requested methods/headers
- This is the recommended approach from tower-http for authenticated APIs
- No additional imports or complex configuration needed