# Improve ServerConfig with Configuration Struct

**Date**: 2025-09-03@11:21  
**Status**: Draft

## Problem Statement

The current `ServerConfig::new()` method takes 6 unnamed positional arguments, which has several issues:

1. **Poor Developer Experience**: Arguments must be provided in exact order
2. **Error Prone**: Easy to mix up parameters of the same type (e.g., `String` parameters)
3. **Inflexible**: All parameters are required, even when defaults would be sensible
4. **Hard to Read**: Code like `ServerConfig::new(3000, "data/production.db".to_string(), secret, ".yourdomain.com".to_string(), false, false)` is unclear
5. **Breaking Changes**: Adding new parameters requires updating all call sites

Current usage from README:
```rust
let config = ServerConfig::new(
    3000, // port
    "data/production.db".to_string(), // sqlite_file_path
    your_32_byte_secret, // session_secret (Vec<u8>)
    ".yourdomain.com".to_string(), // cookie_domain
    false, // insecure_cookie
    false, // development_mode
)?;
```

## Proposed Solution

Use **separate methods for each optional parameter** - the simplest Rust-idiomatic approach:

1. **Required Constructor**: Takes only the required parameter
2. **Optional Setters**: Chainable methods for optional parameters
3. **Flexible Ordering**: Methods can be chained in any order
4. **No Boilerplate**: No `Option<T>`, no `..Default::default()`, no macros
5. **Validation**: Same validation as current implementation
6. **Rust Idiomatic**: Uses the standard builder-like pattern without complexity

## Implementation Plan

### Phase 1: Simple Constructor + Chainable Setters

Replace the complex constructor with a simple approach:

```rust
// src/config.rs

impl ServerConfig {
    /// Create a new ServerConfig with only the required session_secret
    /// All other parameters use sensible defaults
    pub fn new(session_secret: Vec<u8>) -> Result<Self, ConfigError> {
        // Validate session secret length
        if session_secret.len() != 32 {
            return Err(ConfigError::InvalidSecretLength {
                actual: session_secret.len(),
                expected: 32,
            });
        }

        Ok(Self {
            port: 3000,
            sqlite_file_path: "data/development.db".to_string(),
            session_secret,
            cookie_domain: ".localhost".to_string(),
            insecure_cookie: false,
            development_mode: false,
        })
    }

    /// Set the server port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the SQLite database file path
    pub fn with_sqlite_file_path<S: Into<String>>(mut self, path: S) -> Self {
        self.sqlite_file_path = path.into();
        self
    }

    /// Set the cookie domain
    pub fn with_cookie_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.cookie_domain = domain.into();
        self
    }

    /// Enable insecure cookies for development
    pub fn with_insecure_cookie(mut self, insecure: bool) -> Self {
        self.insecure_cookie = insecure;
        self
    }

    /// Enable development mode
    pub fn with_development_mode(mut self, dev_mode: bool) -> Self {
        self.development_mode = dev_mode;
        self
    }
}
```

### Phase 2: Update Public API

No changes needed to `src/lib.rs` - just using the existing `ServerConfig` export.

## Usage Examples

### Quick Setup with Defaults
```rust
// Just the required parameter - everything else uses defaults
let config = ServerConfig::new(your_secret)?;
```

### Production Configuration
```rust
use dp_auth_service::{start_server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::new(your_32_byte_secret)?
        .with_sqlite_file_path("data/production.db")
        .with_cookie_domain(".yourdomain.com");

    start_server(config).await
}
```

### Development Configuration
```rust
let config = ServerConfig::new(dev_secret)?
    .with_port(8080)
    .with_development_mode(true)
    .with_insecure_cookie(true);
```

### Flexible Parameter Ordering
```rust
let config = ServerConfig::new(secret)?
    .with_development_mode(true)
    .with_port(3001)
    .with_cookie_domain(".dev.localhost");
```

## Files to be Created/Modified

### New Files
- None (all changes in existing files)

### Modified Files

1. **`src/config.rs`**
   - Add `ServerConfigOptions` struct with optional fields
   - Replace `ServerConfig::new()` method to accept `ServerConfigOptions`
   - Keep same validation logic

2. **`src/lib.rs`**
   - Export `ServerConfigOptions` in public API

3. **`scripts/run_local_server.rs`**
   - Update to use new `ServerConfigOptions` struct

4. **`README.md`** (documentation update)
   - Update examples to show new struct-based approach

## Benefits

1. **Improved Readability**: Named fields clearly indicate what each parameter does
2. **Flexible Ordering**: Struct fields can be specified in any order
3. **Optional Parameters**: Only session_secret is required, others have sensible defaults
4. **Simple Implementation**: Much simpler than builder pattern - just a struct with optional fields
5. **Better Defaults**: Sensible defaults reduce boilerplate for common cases
6. **Validation**: Same validation logic as before
7. **Future-Proof**: Easy to add new optional parameters without breaking changes
8. **Rust Idiomatic**: Uses standard Rust patterns (Option<T>, Default trait)

## Breaking Changes

- `ServerConfig::new()` signature changes from 6 positional parameters to single `ServerConfigOptions` parameter
- All existing code using `ServerConfig::new()` will need to be updated
- This is acceptable since breaking changes were explicitly allowed

This approach provides a much simpler and cleaner API while solving all the original problems.