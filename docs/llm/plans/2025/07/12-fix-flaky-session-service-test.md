# Fix Flaky Session Service Test

**Date**: 2025-07-12@08:22
**Task**: Fix the flaky test `test_encode_decode_with_different_keys` that occasionally fails in CI

## Problem Analysis

The test `services::session_service::tests::test_encode_decode_with_different_keys` is failing with:
```
called `Result::unwrap()` on an `Err` value: InvalidSecretKey("Key too short: 5 bytes, need at least 32")
```

### Root Cause

Looking at the test code (lines 489-514), the issue is a **race condition** in environment variable management:

1. The test saves the original key: `let original_key = env::var("DP_AUTH_SECRET_KEY").ok();`
2. Sets a first key for encoding: `env::set_var("DP_AUTH_SECRET_KEY", "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=");`
3. Encodes a token: `let token = SessionService::encode_token(&payload).unwrap();`
4. Sets a different key for decoding: `env::set_var("DP_AUTH_SECRET_KEY", "465c4b/KryoNecAbP6TsNqO5L8CYb28bID+MgiK5Y+o=");`
5. Tries to decode: `let result = SessionService::decode_token(&token);`

**The problem**: When tests run in parallel (which they do in CI), other tests might be calling `setup_test_key()` which sets the environment variable to a different value. Specifically, `setup_test_key()` sets it to `"QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg="`, but the error shows "5 bytes", which suggests the base64 decoding of some corrupted/partial value is happening.

The flakiness occurs because:
- Environment variables are **global process state**
- Tests run in parallel threads sharing the same process
- Multiple tests are modifying `DP_AUTH_SECRET_KEY` simultaneously
- Race conditions cause one test to read a partially-set or wrong value

## Proposed Solutions

### Option 1: Test Isolation with Mutex (Recommended)
Use a test-specific mutex to serialize access to the environment variable for tests that need to modify it.

**Pros**: 
- Minimal code changes
- Preserves existing test logic
- Prevents race conditions

**Cons**: 
- Tests that modify env vars run sequentially (slower)
- Still uses global state

### Option 2: Refactor to Accept Key Parameter
Modify `SessionService` methods to optionally accept a key parameter instead of always reading from environment.

**Pros**: 
- Eliminates global state dependency in tests
- More testable design
- Tests can run in parallel

**Cons**: 
- Requires API changes to production code
- More invasive refactoring

### Option 3: Use Test-Specific Environment Variables
Use different environment variable names for different tests.

**Pros**: 
- Avoids conflicts between tests
- Minimal changes

**Cons**: 
- Still uses global state
- Requires modifying production code to accept different env var names

## Recommended Implementation: Option 1

I recommend **Option 1** because it's the least invasive and maintains the current API while fixing the race condition.

### Implementation Details

1. **Add a test mutex**: Create a static mutex for synchronizing environment variable access
2. **Wrap env var modifications**: Use the mutex around any test that modifies `DP_AUTH_SECRET_KEY`
3. **Ensure proper cleanup**: Make sure environment variables are always restored

### Code Changes

#### File: `src/services/session_service.rs`

Add at the top of the test module:
```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::database::test_utils::create_test_database;
  use crate::services::UserService;
  use std::env;
  use std::sync::Mutex;
  
  // Mutex to serialize tests that modify environment variables
  static ENV_MUTEX: Mutex<()> = Mutex::new(());
  
  // ... rest of tests
}
```

Modify affected tests to use the mutex:
```rust
#[test]
fn test_encode_decode_with_different_keys() {
  let _guard = ENV_MUTEX.lock().unwrap();
  let original_key = env::var("DP_AUTH_SECRET_KEY").ok();

  // Encode with one key (32 bytes)
  env::set_var(
    "DP_AUTH_SECRET_KEY",
    "QvQlwpMujK+qzdRbUCikjc131OKt1KHE38Yq37V0Tbg=",
  );
  let payload = SessionService::create_payload(123);
  let token = SessionService::encode_token(&payload).unwrap();

  // Try to decode with different key (also 32 bytes)
  env::set_var(
    "DP_AUTH_SECRET_KEY",
    "465c4b/KryoNecAbP6TsNqO5L8CYb28bID+MgiK5Y+o=",
  );
  let result = SessionService::decode_token(&token);

  // Restore original key if it existed
  if let Some(key) = original_key {
    env::set_var("DP_AUTH_SECRET_KEY", key);
  } else {
    env::remove_var("DP_AUTH_SECRET_KEY");
  }

  assert!(matches!(result, Err(SessionError::DecodingError(_))));
}
```

Similar changes needed for:
- `test_missing_secret_key()`
- `test_invalid_secret_key()`

### Alternative: RAII Guard Pattern

For cleaner code, we could create a helper struct:
```rust
struct EnvVarGuard {
  key: String,
  original_value: Option<String>,
  _guard: std::sync::MutexGuard<'static, ()>,
}

impl EnvVarGuard {
  fn new(key: &str, value: &str) -> Self {
    let _guard = ENV_MUTEX.lock().unwrap();
    let original_value = env::var(key).ok();
    env::set_var(key, value);
    Self {
      key: key.to_string(),
      original_value,
      _guard,
    }
  }
}

impl Drop for EnvVarGuard {
  fn drop(&mut self) {
    if let Some(value) = &self.original_value {
      env::set_var(&self.key, value);
    } else {
      env::remove_var(&self.key);
    }
  }
}
```

## Testing Strategy

1. **Verify the fix**: Run the specific failing test multiple times to ensure it's stable
2. **Parallel test verification**: Run the full test suite with `--test-threads=1` and with default parallelism
3. **CI verification**: Ensure the fix works in the CI environment where the flakiness was observed

## Files to Modify

- `src/services/session_service.rs` - Add mutex and modify affected tests

## Success Criteria

- [ ] `test_encode_decode_with_different_keys` passes consistently
- [ ] All other session service tests continue to pass
- [ ] No performance regression in test execution
- [ ] CI builds are stable