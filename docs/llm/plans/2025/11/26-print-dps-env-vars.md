# Print DPS_ Environment Variables on Server Startup

## Date
2025-11-26@14:45

## Overview
Add functionality to the `run_local_server` script to print all environment variables that start with `DPS_` when the server starts. This will help with debugging configuration issues during local development.

## Implementation Details

### Files to Modify
- `scripts/run_local_server.rs` - Add environment variable printing logic

### Code Changes

#### Location in run_local_server.rs
Add the code after line 8 (after `dotenvy::dotenv().ok()`) and before line 10 (before logging initialization).

#### Code to Add
```rust
// Print DPS_ environment variables for debugging
println!("=== DPS Configuration Variables ===");
for (key, value) in std::env::vars() {
    if key.starts_with("DPS_") {
        println!("{}: {}", key, value);
    }
}
println!("====================================\n");
```

### Rationale
1. **Placement**: After `.env` loading but before logging initialization ensures we capture all loaded variables
2. **Format**: Clear section headers make the output easy to read and distinguish from other logs
3. **Filtering**: Only shows `DPS_` prefixed variables to avoid noise from other environment variables
4. **Simple approach**: Uses standard library `std::env::vars()` without adding new dependencies

### Expected Output Example
```
=== DPS Configuration Variables ===
DPS_AUTH_SECRET: my-secret-key
DPS_DATABASE_URL: sqlite:./data/auth.db
DPS_LOG_LEVEL: debug
DPS_PORT: 8080
====================================
```

## Benefits
- Easy debugging of configuration issues during local development
- Clear visibility into which environment variables are actually loaded
- No impact on production behavior (only in local development script)
- No additional dependencies required