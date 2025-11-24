# GraphQL Field Casing Standardization Plan

## Analysis Summary

After analyzing all GraphQL resolvers, I've identified significant casing inconsistencies across input and output fields. The current mix of snake_case and camelCase creates an inconsistent API experience.

## Current Field Casing Issues

### Output Fields (Response Objects)
**snake_case fields (consistent):**
- `user_id`, `role_id`, `created_ts`, `updated_ts` (AuthRegisterResponse, AuthMeResponse)
- `session_iat`, `session_exp` (AuthMeResponse)
- `metadata_json` (AddSiteResponse, UpdateSiteResponse, RemoveSiteResponse)

**camelCase fields in GraphQL queries (inconsistent):**
- `userId`, `roleId`, `createdTs`, `updatedTs` (in test queries)
- `sessionIat`, `sessionExp` (in test queries)
- `metadataJson` (in test queries)

### Input Fields (Mutation Parameters)
**snake_case fields (consistent):**
- `current_password`, `new_password`, `new_password_confirmation` (auth_change_password)
- `password_confirmation` (auth_register)
- `metadata_json` (add_site, update_site)

**camelCase fields in GraphQL queries (inconsistent):**
- `currentPassword`, `newPassword`, `newPasswordConfirmation` (in test queries)
- `passwordConfirmation` (in test queries)
- `metadataJson` (in test queries)

### Mutation/Query Names
**snake_case in Rust code, camelCase in GraphQL:**
- `auth_register` → `authRegister`
- `auth_login` → `authLogin`
- `auth_change_password` → `authChangePassword`
- `auth_logout` → `authLogout`
- `auth_me` → `authMe`
- `add_site` → `addSite`
- `update_site` → `updateSite`
- `remove_site` → `removeSite`
- `get_server_timestamp` → `getServerTimestamp`

## Standardization Decision

**Target: camelCase for all GraphQL fields (standard GraphQL convention)**

GraphQL best practices and the broader GraphQL ecosystem use camelCase for field names. While Rust uses snake_case, async-graphql automatically handles the conversion when we use the `#[graphql(name = "...")]` attribute.

## Implementation Plan

### Phase 1: Update Response Structs
For each response struct, add GraphQL name attributes to convert snake_case Rust fields to camelCase GraphQL fields:

**Files to modify:**
- `src/graphql/resolvers/auth_register.rs` - AuthRegisterResponse
- `src/graphql/resolvers/auth_login.rs` - AuthLoginResponse  
- `src/graphql/resolvers/auth_change_password.rs` - AuthChangePasswordResponse
- `src/graphql/resolvers/auth_logout.rs` - AuthLogoutResponse
- `src/graphql/resolvers/auth_me.rs` - AuthMeResponse
- `src/graphql/resolvers/add_site.rs` - AddSiteResponse
- `src/graphql/resolvers/update_site.rs` - UpdateSiteResponse
- `src/graphql/resolvers/remove_site.rs` - RemoveSiteResponse

**Example changes:**
```rust
#[derive(async_graphql::SimpleObject)]
pub struct AuthRegisterResponse {
  #[graphql(name = "userId")]
  pub user_id: i64,
  #[graphql(name = "roleId")]
  pub role_id: i64,
  #[graphql(name = "createdTs")]
  pub created_ts: i64,
  #[graphql(name = "updatedTs")]
  pub updated_ts: i64,
  // ... other fields
}
```

### Phase 2: Update Mutation/Query Resolver Methods
Add GraphQL name attributes to convert snake_case Rust method names to camelCase GraphQL field names:

**Files to modify:**
- All resolver files for consistent naming

**Example changes:**
```rust
#[Object]
impl AuthRegisterResolver {
  #[graphql(name = "authRegister")]
  async fn auth_register(
    &self,
    ctx: &Context<'_>,
    #[graphql(name = "username")] username: String,
    #[graphql(name = "password")] password: String,
    #[graphql(name = "passwordConfirmation")] password_confirmation: String,
  ) -> Result<AuthRegisterResponse>
```

### Phase 3: Update Test Queries
Update all GraphQL test queries to use camelCase consistently:

**Files to modify:**
- All resolver test sections

**Example changes:**
```graphql
mutation {
  authRegister(username: "testuser", password: "testpass123", passwordConfirmation: "testpass123") {
    userId
    uuid
    username
    roleId
    createdTs
    updatedTs
    message
  }
}
```

## Detailed Field Mapping

### Auth Fields
| Rust Field | GraphQL Field |
|------------|---------------|
| user_id | userId |
| role_id | roleId |
| created_ts | createdTs |
| updated_ts | updatedTs |
| session_iat | sessionIat |
| session_exp | sessionExp |
| current_password | currentPassword |
| new_password | newPassword |
| new_password_confirmation | newPasswordConfirmation |
| password_confirmation | passwordConfirmation |

### Site Fields
| Rust Field | GraphQL Field |
|------------|---------------|
| metadata_json | metadataJson |

### Resolver Names
| Rust Method | GraphQL Field |
|-------------|---------------|
| auth_register | authRegister |
| auth_login | authLogin |
| auth_change_password | authChangePassword |
| auth_logout | authLogout |
| auth_me | authMe |
| add_site | addSite |
| update_site | updateSite |
| remove_site | removeSite |
| sites | sites (already camelCase) |
| get_server_timestamp | getServerTimestamp |

## Benefits

1. **Consistency**: All GraphQL fields follow camelCase convention
2. **Standards Compliance**: Aligns with GraphQL ecosystem best practices
3. **Developer Experience**: Predictable API for frontend developers
4. **Tooling Compatibility**: Better integration with GraphQL code generation tools

## Testing Strategy

1. Run existing tests to ensure they pass with updated field names
2. Verify GraphQL schema reflects camelCase fields
3. Test both queries and mutations with new field names
4. Ensure error messages and responses use correct casing

## Phase 4: Update README Documentation

**File to modify:**
- `README.md` - Update GraphQL Operations tables

**Required changes:**
- Update all field names in the GraphQL Operations tables to use camelCase
- Ensure consistency between documentation and actual GraphQL schema

**Specific updates needed:**
- Line 32: `user_id (Int), uuid (String), username (String), role_id (Int), created_ts (Int), updated_ts (Int), session_iat (Int), session_exp (Int)` → `userId (Int), uuid (String), username (String), roleId (Int), createdTs (Int), updatedTs (Int), sessionIat (Int), sessionExp (Int)`
- Line 39: `user_id (Int), uuid (String), username (String), role_id (Int), created_ts (Int), updated_ts (Int), message (String)` → `userId (Int), uuid (String), username (String), roleId (Int), createdTs (Int), updatedTs (Int), message (String)`
- Line 40: `token (String), user_id (Int), username (String), message (String)` → `token (String), userId (Int), username (String), message (String)`
- Line 43: `id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), created_ts (Int), updated_ts (Int)` → `id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int)`
- Line 44: `id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), created_ts (Int), updated_ts (Int)` → `id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int)`
- Line 45: `site_id (Int!)` → `siteId (Int!)` and `id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), created_ts (Int), updated_ts (Int)` → `id (Int), slug (String), subdomain (String), port (Int), protocol (String), metadataJson (String), createdTs (Int), updatedTs (Int)`

## Notes

- Internal Rust code keeps snake_case (Rust conventions)
- Only GraphQL schema surface changes to camelCase
- async-graphql handles the conversion automatically
- Database schema and models remain unchanged
- This is a breaking change for GraphQL API consumers
- README documentation must be updated to reflect new field names