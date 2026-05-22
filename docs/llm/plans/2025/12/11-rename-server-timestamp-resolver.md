# Plan: Rename getServerTimestamp Resolver to serverTimestamp

## Overview
This plan renames the `getServerTimestamp` GraphQL resolver to `serverTimestamp` to follow camelCase naming conventions. This involves updating the resolver struct name, filename, GraphQL field name, and all references throughout the codebase.

## Files to Modify

### 1. Rename Resolver File
- **From**: `src/graphql/resolvers/get_server_timestamp.rs`
- **To**: `src/graphql/resolvers/server_timestamp.rs`

**Changes needed:**
- Rename struct from `GetServerTimestampResolver` to `ServerTimestampResolver`
- Update GraphQL field name from `getServerTimestamp` to `serverTimestamp`
- Update function name from `get_server_timestamp` to `server_timestamp`
- Update test function names to match new naming
- Update GraphQL query strings in tests from `{ getServerTimestamp }` to `{ serverTimestamp }`

### 2. Update Module Exports
- **File**: `src/graphql/resolvers/mod.rs`
- **Changes:**
  - Update module declaration: `pub mod get_server_timestamp;` → `pub mod server_timestamp;`
  - Update export: `pub use get_server_timestamp::GetServerTimestampResolver;` → `pub use server_timestamp::ServerTimestampResolver;`

### 3. Update Schema Registration
- **File**: `src/graphql/schema.rs`
- **Changes:**
  - Update import: `GetServerTimestampResolver` → `ServerTimestampResolver`
  - Update Query struct field: `GetServerTimestampResolver` → `ServerTimestampResolver`
  - Update documentation comment: `getServerTimestamp` → `serverTimestamp`

### 4. Update README Documentation
- **File**: `README.md`
- **Changes:**
  - Update GraphQL operations table: `getServerTimestamp` → `serverTimestamp`

## Implementation Details

### New Resolver Structure
```rust
/// Server timestamp query resolver providing current server time information
#[derive(Default, Debug)]
pub struct ServerTimestampResolver;

#[Object]
impl ServerTimestampResolver {
  /// Returns the current server timestamp in milliseconds since Unix epoch.
  #[instrument]
  #[graphql(name = "serverTimestamp")]
  async fn server_timestamp(&self) -> Result<String> {
    let timestamp = ServerService::get_server_timestamp();
    Ok(timestamp.to_string())
  }
}
```

### Updated Test Structure
```rust
#[tokio::test]
async fn test_server_timestamp_calls_service() {
  let query = ServerTimestampResolver;
  let schema = Schema::build(query, EmptyMutation, EmptySubscription).finish();
  let result = schema.execute("{ serverTimestamp }").await;
  // ... assertions remain the same
}

#[tokio::test]
async fn test_server_timestamp_returns_string() {
  let query = ServerTimestampResolver;
  let schema = Schema::build(query, EmptyMutation, EmptySubscription).finish();
  let result = schema.execute("{ serverTimestamp }").await;
  // ... assertions remain the same
}
```

## Steps to Implement

1. **Rename the resolver file** from `get_server_timestamp.rs` to `server_timestamp.rs`
2. **Update the resolver implementation** with new struct and function names
3. **Update module exports** in `mod.rs`
4. **Update schema imports and documentation** in `schema.rs`
5. **Update README documentation** to reflect the new query name
6. **Run tests** to ensure all functionality works correctly
7. **Run linter** to fix any formatting issues

## Testing Requirements

- All existing unit tests should pass with updated query names
- GraphQL schema should correctly expose `serverTimestamp` query
- Integration tests should work with the new query name
- No functional changes to the timestamp logic itself

## Notes

- This is a purely cosmetic change that improves naming consistency
- The underlying `ServerService::get_server_timestamp()` method remains unchanged
- All functionality and return types stay the same
- This change maintains backward compatibility at the service layer while updating the GraphQL interface