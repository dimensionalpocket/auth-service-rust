# Refactor GraphQL Mutations to Use Direct Fields Instead of Input Objects

**Date**: 2025-11-24@14:50  
**Status**: Planning

## Analysis

The current GraphQL mutations use input object patterns for several operations. The request is to change these to use direct fields for consistency and better developer experience.

### Current Implementation Analysis

After reviewing all resolvers, I found the following input object patterns:

1. **`authLogin`** - Uses `AuthLoginInput` with username/password
2. **`authRegister`** - Uses `AuthRegisterInput` with username/password/passwordConfirmation
3. **`authChangePassword`** - Uses `AuthChangePasswordInput` with currentPassword/newPassword/newPasswordConfirmation
4. **`addSite`** - Uses `AddSiteInput` with slug/subdomain/port/protocol/metadataJson
5. **`updateSite`** - Uses `UpdateSiteInput` with id/slug/subdomain/port/protocol/metadataJson

### Resolvers Already Using Direct Fields

The following resolvers already use direct fields and don't need changes:
- **`authLogout`** - No parameters
- **`authMe`** - No parameters (uses session context)
- **`sites`** - No parameters
- **`getServerTimestamp`** - No parameters
- **`removeSite`** - Uses direct `site_id: i64` parameter

### Files Requiring Changes

1. **`src/graphql/resolvers/auth_login.rs`** - Remove `AuthLoginInput`, update method signature
2. **`src/graphql/resolvers/auth_register.rs`** - Remove `AuthRegisterInput`, update method signature
3. **`src/graphql/resolvers/auth_change_password.rs`** - Remove `AuthChangePasswordInput`, update method signature
4. **`src/graphql/resolvers/add_site.rs`** - Remove `AddSiteInput`, update method signature
5. **`src/graphql/resolvers/update_site.rs`** - Remove `UpdateSiteInput`, update method signature
6. **`README.md`** - Update documentation to reflect direct fields usage

### Implementation Plan

#### Step 1: Update AuthLoginResolver
- Remove `AuthLoginInput` struct definition
- Change resolver method signature from:
  ```rust
  async fn auth_login(&self, ctx: &Context<'_>, input: AuthLoginInput)
  ```
  to:
  ```rust
  async fn auth_login(&self, ctx: &Context<'_>, username: String, password: String)
  ```
- Update instrumentation macro and service calls accordingly
- Update all test queries to use direct fields

#### Step 2: Update AuthRegisterResolver
- Remove `AuthRegisterInput` struct definition
- Change resolver method signature from:
  ```rust
  async fn auth_register(&self, ctx: &Context<'_>, input: AuthRegisterInput)
  ```
  to:
  ```rust
  async fn auth_register(&self, ctx: &Context<'_>, username: String, password: String, password_confirmation: String)
  ```
- Update instrumentation macro and service calls accordingly
- Update all test queries to use direct fields

#### Step 3: Update AuthChangePasswordResolver
- Remove `AuthChangePasswordInput` struct definition
- Change resolver method signature from:
  ```rust
  async fn auth_change_password(&self, ctx: &Context<'_>, input: AuthChangePasswordInput)
  ```
  to:
  ```rust
  async fn auth_change_password(&self, ctx: &Context<'_>, current_password: String, new_password: String, new_password_confirmation: String)
  ```
- Update service calls accordingly
- Update all test queries to use direct fields

#### Step 4: Update AddSiteResolver
- Remove `AddSiteInput` struct definition
- Change resolver method signature from:
  ```rust
  async fn add_site(&self, ctx: &Context<'_>, input: AddSiteInput)
  ```
  to:
  ```rust
  async fn add_site(&self, ctx: &Context<'_>, slug: String, subdomain: Option<String>, port: Option<i64>, protocol: Option<String>, metadata_json: Option<String>)
  ```
- Update instrumentation macro and service calls accordingly
- Update all test queries to use direct fields

#### Step 5: Update UpdateSiteResolver
- Remove `UpdateSiteInput` struct definition
- Change resolver method signature from:
  ```rust
  async fn update_site(&self, ctx: &Context<'_>, input: UpdateSiteInput)
  ```
  to:
  ```rust
  async fn update_site(&self, ctx: &Context<'_>, id: i64, slug: Option<String>, subdomain: Option<String>, port: Option<i64>, protocol: Option<String>, metadata_json: Option<String>)
  ```
- Update instrumentation macro and service calls accordingly
- Update all test queries to use direct fields

#### Step 6: Update Documentation
- Update README.md to reflect direct fields usage for all affected mutations
- Ensure consistency between documentation and implementation

### Benefits of This Change

1. **Simpler GraphQL schema** - Reduces unnecessary nesting for simple mutations
2. **Better developer experience** - Less verbose mutation calls
3. **Consistency** - All mutations will follow the same pattern (direct fields)
4. **Follows GraphQL best practices** - Direct fields are preferred for simple inputs
5. **Maintains functionality** - All business logic remains unchanged

### Risks and Considerations

1. **Breaking change** - Existing clients will need to update their GraphQL queries
2. **Test coverage** - All tests must be updated to reflect new format
3. **Documentation consistency** - Ensure all docs reflect the change
4. **Parameter order** - Must maintain logical parameter ordering in method signatures

### GraphQL Schema Impact Examples

Before:
```graphql
input AuthLoginInput {
  username: String!
  password: String!
}

input AddSiteInput {
  slug: String!
  subdomain: String
  port: Int
  protocol: String
  metadataJson: String
}

type Mutation {
  authLogin(input: AuthLoginInput!): AuthLoginResponse
  addSite(input: AddSiteInput!): AddSiteResponse
}
```

After:
```graphql
type Mutation {
  authLogin(username: String!, password: String!): AuthLoginResponse
  addSite(slug: String!, subdomain: String, port: Int, protocol: String, metadataJson: String): AddSiteResponse
}
```

### Implementation Notes

- No changes needed to services, orchestrators, or database layer
- All business logic remains the same
- Response types remain unchanged
- Authentication and authorization logic is unaffected
- Cookie handling logic remains unchanged
- Parameter validation logic remains the same

## TODO

- [ ] Implement refactoring for all 5 affected resolvers
- [ ] Update all test queries to use direct fields
- [ ] Update README.md documentation
- [ ] Run all tests to ensure they pass with new format
- [ ] Run linter to ensure code quality
- [ ] TODO: Update README.md mutations/queries table to reflect direct fields usage