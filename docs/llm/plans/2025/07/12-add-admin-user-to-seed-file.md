# Add Admin User to Seed File

**Date**: 2025-07-12@10:20  
**Task**: Add an admin user with username "admin" and password "Ch4nG3M3!" to the database seed file.

## Overview

This plan adds a default admin user to the seed data that will be created during database initialization. The user will have the admin role and a hashed password. The password will be changed in production.

## Analysis

From examining the current codebase:

1. **Seed System**: The project has a seeding system in `config/database/seeds/` that runs SQL files in alphabetical order
2. **Current Seeds**: Only `001_default_roles.sql` exists, which creates 'admin' and 'user' roles
3. **Password Hashing**: The `PasswordService` uses Argon2 with 16-byte salt for secure password hashing
4. **User Creation**: Users are created with UUID, name, role_id, password_hash, and timestamps
5. **Database Schema**: Users table has columns: id, uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json

## Implementation Plan

### 1. Create New Seed File

**File**: `config/database/seeds/002_default_users.sql`

This file will:
- Insert the admin user with a pre-hashed password
- Use the admin role (role_id = 1, based on the current seed order)
- Generate a UUID for the user
- Set appropriate timestamps
- Use `INSERT OR IGNORE` for idempotency

### 2. Generate Password Hash

Since we need to insert a pre-hashed password, we'll need to:
- Use the existing `PasswordService::generate` method to hash "Ch4nG3M3!"
- Include the generated hash in the seed file
- Document that this password should be changed in production

### 3. Implementation Steps

1. **Generate the password hash** using the existing password service
2. **Create the seed file** with the admin user data
3. **Test the seeding process** to ensure it works correctly
4. **Verify the user can be retrieved** after seeding

## Files to Create/Modify

### New Files

1. `config/database/seeds/002_default_users.sql` - New seed file for default users

### Code Samples

**Seed File Content** (`config/database/seeds/002_default_users.sql`):
```sql
-- Insert default users
-- Using INSERT OR IGNORE to make seeds idempotent (safe to run multiple times)
INSERT OR IGNORE INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES 
    (
        '550e8400-e29b-41d4-a716-446655440000', -- Fixed UUID for admin user
        strftime('%s', 'now'), 
        strftime('%s', 'now'), 
        'admin', 
        (SELECT id FROM user_roles WHERE name = 'admin' LIMIT 1), -- Get admin role_id dynamically
        '[GENERATED_HASH_WILL_BE_INSERTED_HERE]', -- Hash of "Ch4nG3M3!"
        NULL
    );
```

**Password Hash Generation** (temporary script to generate the hash):
```rust
// This will be a temporary script to generate the hash
use crate::services::PasswordService;

fn main() {
    let password = "Ch4nG3M3!";
    match PasswordService::generate(password) {
        Ok(hash) => println!("Generated hash: {}", hash),
        Err(e) => eprintln!("Error generating hash: {}", e),
    }
}
```

## Testing Strategy

1. **Run the seeding process** and verify the admin user is created
2. **Test user retrieval** by username to confirm the user exists
3. **Test password verification** to ensure the hash works correctly
4. **Verify idempotency** by running seeds multiple times
5. **Integration test** to ensure the user can authenticate

## Security Considerations

1. **Production Password Change**: Document that the default password must be changed in production
2. **Hash Security**: Use the existing Argon2 implementation for secure hashing
3. **UUID Consistency**: Use a fixed UUID for the admin user to ensure consistency across environments

## Success Criteria

- [ ] Admin user is created during database seeding
- [ ] User has username "admin" and admin role
- [ ] Password "Ch4nG3M3!" can be verified against the stored hash
- [ ] Seeding process remains idempotent
- [ ] User can be retrieved by username after seeding
- [ ] Integration tests pass with the new admin user

## Notes

- The password "Ch4nG3M3!" is temporary and should be changed in production
- The UUID `550e8400-e29b-41d4-a716-446655440000` is used for consistency
- The admin role_id is retrieved dynamically using a subquery to ensure correctness
- This follows the existing pattern of using `INSERT OR IGNORE` for idempotency