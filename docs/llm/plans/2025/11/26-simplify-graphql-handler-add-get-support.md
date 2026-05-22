# Plan: Simplify GraphQL Handler and Add GET Support

## Overview
Replace the overly complex custom GraphQL handler (224 lines) with standard async-graphql-axum patterns, reducing code by ~90% while adding proper GET query support per GraphQL specification.

## Current Issues
1. **Missing GET Support**: GraphQL GET requests return "Method not allowed" instead of handling queries via URL parameters
2. **Code Bloat**: Custom handler reimplements functionality already provided by `async-graphql-axum::GraphQL`
3. **Reinvented Wheel**: Manual request parsing, error handling, and response formatting that the library already handles

## Proposed Solution

### Step 1: Create Custom GraphQL Extractor
- Create custom extractor that combines `GraphQLBatchRequest` with existing session context
- Extract GraphQL request using existing `async-graphql-axum` functionality
- Leverage existing session middleware (already adds `SessionContext` to request extensions)
- Add session from request extensions to GraphQL request data field
- Config data will be injected at service level (static, doesn't change per request)
- Maintain existing error handling patterns

### Step 2: Update Router Configuration
- Use service-based routing with `GraphQL::new(schema)` for efficiency
- Existing session middleware already ensures session is available in request extensions
- Support both GET and POST methods automatically via library
- Keep playground functionality separate from actual GraphQL operations

### Step 2: Update Router Configuration
- Modify `build_router()` in `dps_auth_api.rs`
- Support both GET and POST methods on `/graphql` endpoint
- Keep playground functionality separate from actual GraphQL operations

### Step 4: Cleanup
- Remove custom request parsing functions (`parse_graphql_request`)
- Remove unused imports and helper functions
- Remove old `graphql_post_handler` function (replaced by service-based routing)

## Implementation Details

### Simplified GraphQL Handler (Uses GraphQLRequest Extractor)
```rust
// src/handlers/graphql.rs
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::Extension;

async fn graphql_handler(
    // 1. Extract the schema (injected at startup)
    Extension(schema): Extension<Schema>,
    // 2. Extract session from existing middleware
    Extension(session): Extension<SessionContext>,
    // 3. Extractor automatically handles GET (query params) and POST (body)
    req: GraphQLRequest,
) -> GraphQLResponse {
    // Inject session into GraphQL request context (3 lines of logic!)
    let request = req.into_inner().data(session);
    
    // Execute and return response
    schema.execute(request).await.into()
}
```

### Updated Router Configuration
```rust
// In build_router()
.route(
    &format!("{}/graphql", self.config.api_path),
    // Same handler handles both GET and POST automatically
    get(graphql_handler)
    .post(graphql_handler)
    // Existing session middleware already adds SessionContext to request extensions
    .layer(from_fn(
        crate::middleware::session::create_session_middleware(self.config.session_secret.clone()),
    ))
)
```

### Schema Creation with Config Injection
```rust
// In create_app() method
let schema = crate::graphql::schema::build_schema()
  .data(database.pool)
  .data(self.config.clone())  // Inject config directly into schema
  .finish();
// Config is now part of schema data, no separate variable needed
```

### Schema Creation with Config Injection
```rust
// In create_app() method
let schema = crate::graphql::schema::build_schema()
  .data(database.pool)
  .data(self.config.clone())  // Inject config directly into schema
  .finish();
let schema_with_config = schema;  // Config is now part of schema data
```

### GET Support Implementation
- `async_graphql_axum::GraphQLRequest` extractor automatically handles GET requests
- Uses `async_graphql::http::parse_query_string()` internally for URL parameter parsing
- Supports standard GraphQL GET format: `?query={...}&operationName={...}&variables={...}`
- Library enforces GraphQL spec (mutations via GET not allowed)

### POST Support Implementation  
- Same `GraphQLRequest` extractor automatically handles POST requests
- Supports JSON body and `application/graphql` content-type
- No manual parsing needed - extractor handles both GET and POST transparently
- Remove custom `parse_graphql_request` function (~50 lines of code)
- Single handler processes both HTTP methods automatically

### Session Context Handling
- Existing session middleware already extracts session and adds to request extensions
- Simplified handler extracts session via `Extension(session): Extension<SessionContext>`
- Session injected into GraphQL context using `req.into_inner().data(session)` (3 lines of code)
- Config data is injected at schema level (static, doesn't change per request)
- No additional middleware needed - leverages existing session infrastructure
- Preserves existing authentication and authorization patterns
- `GraphQLRequest` extractor handles all HTTP method parsing automatically

## Files to Modify

1. **src/handlers/graphql.rs** - Add custom GraphQL extractor, remove custom parsing logic
2. **src/dps_auth_api.rs** - Update router configuration to use service-based routing with config injection
3. **src/graphql/resolvers/auth_login.rs** - Update config access pattern (remove Arc wrapper)
4. **src/graphql/resolvers/auth_logout.rs** - Update config access pattern (remove Arc wrapper)

## Logging Strategy

### Current Logging Implementation
The current handler has comprehensive logging:
- Request timing with `Instant::now()` and `duration.elapsed()`
- Operation name extraction and formatting
- Query whitespace normalization for anonymous operations
- Conditional logging (skip introspection queries)
- Success/error separation with different log levels
- Query content logging for anonymous operations

### New Logging Approach with async-graphql Extensions

**Option 1: Built-in Tracing Extension (Recommended)**
```rust
// In schema creation
let schema = crate::graphql::schema::build_schema()
  .data(database.pool)
  .data(self.config.clone())
  .extension(async_graphql::extensions::Tracing)  // Built-in tracing
  .finish();
```

**Benefits:**
- ✅ Automatic request timing and execution tracing
- ✅ Built-in integration with tracing ecosystem
- ✅ No custom logging code needed
- ✅ Handles all GraphQL lifecycle events
- ✅ Standard tracing format and spans

**Trade-offs:**
- ⚠️ Less control over log format (uses tracing standards)
- ⚠️ No custom whitespace normalization
- ⚠️ No anonymous operation special handling

**Option 2: Custom Extension (Preserve Current Behavior)**
```rust
// Create custom extension to preserve exact current logging behavior
struct CustomLoggingExtension;

impl Extension for CustomLoggingExtension {
    // Replicate current logging logic exactly
}

// In schema creation
.extension(CustomLoggingExtension)
```

### Recommended Approach: Built-in Tracing Extension

Use `async_graphql::extensions::Tracing` for these reasons:

1. **Standards Compliant**: Uses established tracing patterns
2. **Maintenance**: No custom logging code to maintain
3. **Performance**: Optimized by async-graphql team
4. **Features**: Automatic query analysis, field resolution timing
5. **Future-Proof**: Will improve with async-graphql updates

**Sensitive Data Handling:**
- Current implementation uses `#[instrument(skip(self, ctx, password), fields(username = %username))]` for auth operations
- Built-in Tracing extension respects existing tracing instrumentation
- Can continue using `skip` parameter to exclude sensitive data from traces
- Tracing filters and subscribers can control what gets logged

**Log Filtering Availability:**
- Tracing ecosystem provides sophisticated filtering capabilities:
  - Filter by log level (debug, info, error, etc.)
  - Filter by target/module (e.g., only auth-related logs)
  - Filter by span attributes
  - Dynamic filtering via environment variables
  - Structured filtering for specific operations
- More powerful than current manual conditional logic

**Migration Path:**
1. Add `Tracing` extension during schema creation
2. Remove all custom logging code from handler
3. Configure tracing subscriber to match current log output format
4. Test to ensure equivalent observability with better filtering capabilities

## Resolver Changes Required

**Current Pattern**: Resolvers access config via `ctx.data::<Arc<DpsAuthApiConfig>>()?`

**New Pattern**: Resolvers will access config via `ctx.data::<DpsAuthApiConfig>()?` (no Arc wrapper)

### Resolvers Requiring Changes:

1. **auth_login.rs** - Uses config for session_secret, cookie_domain, insecure_cookie, api_path, session_ttl_seconds
2. **auth_logout.rs** - Uses config for cookie_domain, insecure_cookie, api_path

### Example: auth_login.rs Before/After

**Before (Current):**
```rust
let config = ctx.data::<Arc<DpsAuthApiConfig>>()?;
let session_secret = config.session_secret.clone();
let cookie_domain = config.cookie_domain.clone();
let insecure_cookie = config.insecure_cookie;
```

**After (Updated):**
```rust
let config = ctx.data::<DpsAuthApiConfig>()?;
let session_secret = config.session_secret.clone();
let cookie_domain = config.cookie_domain.clone();
let insecure_cookie = config.insecure_cookie;
```

**Note**: Only the type annotation changes from `Arc<DpsAuthApiConfig>` to `DpsAuthApiConfig` since config will be injected directly into schema data rather than wrapped in Arc.

### Why Arc Wrapper is Removed

**Current Implementation (Per-Request):**
```rust
// In handler - config is Arc<DpsAuthApiConfig> passed as parameter
request = request.data(config.clone());  // Arc::clone() per request
```

**New Implementation (Service-Level):**
```rust
// In schema creation - config injected once into schema data
schema.data(self.config.clone())  // Single Arc::clone() at startup
```

**Key Differences:**
1. **Current**: Config is wrapped in Arc because it's passed as parameter to handler and cloned per request
2. **New**: Config is injected directly into schema data at startup, async-graphql handles internal sharing

**Why This Works:**
- **Schema owns the data**: Once injected into schema, async-graphql manages the data lifecycle
- **No per-request cloning**: Config is accessed directly from schema context, not cloned per request
- **Thread safety**: async-graphql ensures thread-safe access to schema data internally
- **Performance**: Eliminates Arc::clone() overhead on every GraphQL request

**Resolver Impact:**
- Resolvers no longer need to unwrap Arc wrapper
- Direct access to config data via `ctx.data::<DpsAuthApiConfig>()`
- Same functionality, better performance

## Testing Strategy

### Functional Tests
1. **GET Queries**: Test `GET /api/graphql?query={getServerTimestamp}`
2. **POST Queries**: Test existing POST query functionality  
3. **POST Mutations**: Test existing POST mutation functionality
4. **Playground**: Verify playground still works in development mode
5. **Error Handling**: Test invalid queries, missing parameters

### Regression Tests
1. Run existing test suite: `cargo test --quiet`
2. Verify all GraphQL operations still work
3. Check session handling still functions
4. Validate authentication flows unchanged

## Benefits

### Code Reduction
- **Request Parsing Function**: 50 lines → 0 lines (100% elimination)
- **Manual Error Handling**: 30 lines → 0 lines (100% elimination)  
- **Custom Logging Logic**: ~40 lines → 0 lines (100% elimination, replaced by Tracing extension)
- **Handler Logic**: ~100 lines → ~10 lines (90% reduction)
- **GraphQLRequest Extractor**: 0 lines (uses existing library extractor)
- **Net Reduction**: ~120 lines removed while adding GET support and improving logging

### Standards Compliance
- ✅ GraphQL GET support via URL parameters
- ✅ Proper HTTP method handling
- ✅ Standard async-graphql-axum patterns

### Maintainability
- Dramatically less custom code to maintain (~10 lines vs 224 lines originally)
- Single handler processes both GET and POST automatically
- Leverages well-tested `GraphQLRequest` extractor for HTTP method handling
- Easier to upgrade async-graphql versions (uses standard patterns)
- Library handles edge cases and spec compliance automatically
- Built-in Tracing extension provides professional-grade observability
- No additional middleware needed - leverages existing session infrastructure
- Clean separation of concerns (session middleware → handler → GraphQLRequest extractor → schema execution)

## Success Criteria

1. ✅ GET queries work via URL parameters
2. ✅ POST operations (queries/mutations) unchanged
3. ✅ Playground works in development mode
4. ✅ All existing tests pass
5. ✅ Session handling unchanged

This plan transforms the GraphQL handler to use a simplified approach that leverages the existing session middleware and the `async-graphql-axum::GraphQLRequest` extractor. The extractor handles GET/POST automatically and efficiently, with a single handler processing both HTTP methods, config data injected at schema level (since it's static), and session context handled via standard Axum extension patterns. This provides both maintainability benefits (90% code reduction) and maintains existing authentication patterns without additional middleware complexity.

## Phased Implementation Strategy

**This is a significant change that should be implemented incrementally to ensure each phase is testable and doesn't break existing functionality.**

### Phase 1: Schema Preparation (Low Risk) ✅ COMPLETED
**Goal**: Prepare schema and resolvers for new approach without changing handler
**Changes**:
1. Update schema creation to inject config data directly
2. Update resolver config access patterns (remove Arc wrapper)
**Files**: `src/dps_auth_api.rs`, `src/graphql/resolvers/auth_login.rs`, `src/graphql/resolvers/auth_logout.rs`
**Testing**: Run existing test suite to ensure no regressions
**Risk**: Low - only data access patterns change

**Implementation Details**:
- Updated `create_app()` to use `.data(self.config.as_ref().clone())` for proper config injection
- Modified `auth_login.rs` and `auth_logout.rs` to access config via `ctx.data::<DpsAuthApiConfig>()` instead of `Arc<DpsAuthApiConfig>`
- Updated `graphql_post_handler` to remove config parameter (now available from schema)
- Fixed router configuration to not pass config to handler
- All 226 tests pass with no regressions
- Performance improvement: eliminated per-request `Arc::clone()` overhead

### Phase 2: Tracing Migration (Low Risk) ✅ COMPLETED
**Goal**: Replace custom logging with Tracing extension
**Changes**:
1. Add Tracing extension to schema creation
2. Remove custom logging code from handler (~40 lines)
3. Verify tracing output matches expectations
4. Test sensitive data filtering works correctly
**Files**: `src/dps_auth_api.rs`, `src/handlers/graphql.rs`
**Testing**: Run tests, verify tracing logs appear, check sensitive data filtering
**Risk**: Low - direct replacement, can rollback if issues

**Implementation Details**:
- Added `tracing` feature to `async-graphql` dependency in `Cargo.toml`
- Updated schema creation with `.extension(async_graphql::extensions::Tracing)`
- Removed all custom logging code from `graphql_post_handler` (~40 lines eliminated)
- Simplified handler from ~75 lines to ~25 lines
- Removed unused imports: `std::time::Instant`, `tracing::{error, info}`, `once_cell::sync::Lazy`, `regex::Regex`
- All 226 tests pass with no regressions
- Sensitive data filtering confirmed via existing `tracing_test::traced_test` tests
- Built-in tracing provides automatic request timing, query analysis, and field resolution timing
- Professional-grade observability without custom maintenance overhead

### Phase 3: Move Playground to GET /playground (Low Risk) ✅ COMPLETED
**Goal**: Separate playground functionality from GraphQL endpoint to simplify next phase
**Rationale**: Moving playground first makes the service-based routing implementation cleaner since we won't need to handle playground logic in the GraphQL service.
**Changes**:
1. Add new route for GET /playground that serves the GraphQL playground only when `development_mode` is enabled
2. Remove playground handling from current GET /graphql endpoint
3. Update router configuration to handle both routes separately
4. Keep existing GraphQL POST handler unchanged for now
5. Preserve existing development mode logic: check if `DEVELOPMENT_MODE` env var equals "Y" (not "true")
**Files**: `src/dps_auth_api.rs`, `src/handlers/graphql.rs`, `tests/playground_tests.rs`
**Testing**: 
- Set `DEVELOPMENT_MODE=Y` and verify playground works at GET /playground
- Set `DEVELOPMENT_MODE` to anything other than "Y" (or unset) and verify GET /playground returns 404
- Verify existing POST /graphql functionality unchanged in all modes
**Risk**: Low - simple route addition and removal

**Implementation Details**:
- Updated `graphql_get_handler()` to only return "Method not allowed" (removed playground logic)
- Added new `playground_handler()` function that serves playground only when `development_mode` is enabled
- Added new route for GET `/api/playground` with development mode check
- Updated GET `/api/graphql` route to use the simplified handler (no playground logic)
- POST `/api/graphql` functionality remains unchanged
- Added comprehensive tests covering all scenarios (development mode enabled/disabled, default config)
- All 226 existing tests pass + 3 new playground-specific tests pass
- Code passes linter and formatter checks with existing development mode logic

### Phase 4: Simplified Handler with GraphQLRequest Extractor (Medium Risk)
**Goal**: Replace complex custom handler with simplified handler using `GraphQLRequest` extractor
**Rationale**: Use `async-graphql-axum::GraphQLRequest` extractor which automatically handles GET (query params) and POST (body) parsing, eliminating need for custom parsing logic.
**Changes**:
1. Replace `graphql_post_handler` with simplified `graphql_handler` using `GraphQLRequest` extractor
2. Update router to use same handler for both GET and POST on /graphql
3. Remove `parse_graphql_request` function (~50 lines eliminated)
4. Inject session context using `req.into_inner().data(session)` pattern
5. Playground already handled separately from Phase 3
**Files**: `src/dps_auth_api.rs`, `src/handlers/graphql.rs`
**Testing**: Full integration test suite, manual testing of GET/POST operations
**Risk**: Medium - handler replacement but uses proven extractor pattern

### Phase 5: Cleanup & Validation (Low Risk)
**Goal**: Remove unused code and validate complete functionality
**Changes**:
1. Remove unused imports and helper functions
2. Remove old handler function
3. Update documentation and comments
**Files**: `src/handlers/graphql.rs`
**Testing**: Complete regression test suite, performance validation
**Risk**: Low - cleanup only

## Implementation Notes

### Incremental Benefits
- **Each phase is independently testable**
- **Can stop at any phase** if issues arise
- **Gradual migration** reduces risk
- **Existing tests validate** each phase
- **Rollback possible** at each phase boundary
- **Phase 2 provides clean validation point** for logging migration

### Testing Strategy
- **Phase 1**: Run `cargo test --quiet` - should pass unchanged
- **Phase 2**: Verify tracing logs appear, test sensitive data filtering, run existing suite (ensure no regressions after removing custom logging)
- **Phase 3**: Add extractor unit tests + existing suite
- **Phase 4**: Manual GET/POST testing + existing suite
- **Phase 5**: Full regression suite + performance tests

### If Incremental Approach Not Possible
If the changes are too interconnected for phased implementation:
1. **Implement in feature branch** with comprehensive testing
2. **Extensive manual testing** before merge
3. **Staged rollout** if deployment allows
4. **Rollback plan** ready if issues detected

This phased approach minimizes risk while allowing validation of each major component before proceeding to the next.