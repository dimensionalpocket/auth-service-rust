# Plan: Add Role Management Resolvers

## Overview
This plan implements a phased approach to add comprehensive role management functionality to the GraphQL API. Each phase implements a specific resolver with proper permission checks, following the established patterns in the codebase.

## Implementation Phases

### Phase 1: roles Query Resolver
**Permission requirement**: `can_manage_roles` OR `can_edit_user_role`

#### Step 1.1: Create/Verify Queries ✅
- Verify `GetAllRolesQuery` exists in `src/queries/user_roles/get_all_roles.rs`
- If missing, create query to fetch all roles with their permissions

#### Step 1.2: Add UserRoleService Methods ✅
- Add `get_all_roles()` method to existing `src/services/user_role_service.rs`
- Return `Vec<UserRole>`
- Handle permission deserialization from JSON to string arrays

#### Step 1.3: Create Orchestrator ✅
- Create `src/orchestrators/role_orchestrator.rs` if it doesn't exist
- Follow `SiteOrchestrator` patterns exactly:
  - Create `RoleError` enum with `DatabaseError(sqlx::Error)`, `AuthenticationError(String)`, `AuthorizationError(String)`
  - Implement `Display`, `Error`, and `From<sqlx::Error>` traits
  - Use `SessionContext` as parameter (not `User` directly)
  - Authentication: Check `session_context.user_id()` and return `AuthenticationError` if None
  - Authorization: Fetch user via `GetUserByIdQuery::run()`, check permissions via `UserRoleService::check_user_permission()`
  - Return `AuthorizationError("Forbidden".to_string())` for unauthorized users
  - Business logic: Call service methods and convert errors appropriately
- Add `get_all_roles_with_permission_check(pool, session_context)` method
- Validate user has either `can_manage_roles` OR `can_edit_user_role` permission
- Call UserRoleService method
- Add comprehensive tests following `SiteOrchestrator` test patterns:
  - Use helper functions `create_test_user()` and `create_test_role()`
  - Use `SessionContext` with `DpsAuthSessionPayload` for authentication
  - Test success, unauthenticated, nonexistent user, and forbidden cases

#### Step 1.4: Create Resolver ✅
- Create `src/graphql/resolvers/roles.rs`
- Define `RoleListing` response type with `permissions` as `Vec<String>` (not `permissions_json`)
- Implement `RolesResolver` with `roles` query
- Add `RolesResolver` to `Query` struct in `src/graphql/schema.rs`
- Add comprehensive tests for success, forbidden, and unauthenticated cases

### Phase 2: role Query Resolver
**Permission requirement**: `can_manage_roles`

#### Step 2.1: Verify Queries ✅
- Verify `GetRoleByIdQuery` exists in `src/queries/user_roles/get_role_by_id.rs`

#### Step 2.2: Add UserRoleService Method ✅
- Add `get_role_by_id(role_id: i64)` method to existing `src/services/user_role_service.rs`
- Return `Option<UserRole>`

#### Step 2.3: Add Orchestrator Method ✅
- Add `get_role_by_id_with_permission_check(pool, session_context, role_id: i64)` to RoleOrchestrator
- Follow `SiteOrchestrator` patterns exactly:
  - Authentication: Check `session_context.user_id()` and return `AuthenticationError` if None
  - Authorization: Fetch user via `GetUserByIdQuery::run()`, check `can_manage_roles` permission
  - Return `AuthorizationError("Forbidden".to_string())` for unauthorized users
  - Business logic: Call service method and handle not found case
- Validate `can_manage_roles` permission
- Call UserRoleService method
- Add tests following `SiteOrchestrator` patterns for success, forbidden, not found, and unauthenticated cases

#### Step 2.4: Create Resolver ✅
- Create `src/graphql/resolvers/role.rs`
- Define `RoleResponse` type with `permissions` as `Vec<String>`
- Implement `RoleResolver` with `role` query taking `id: i64` parameter
- Add `RoleResolver` to `Query` struct in `src/graphql/schema.rs`
- Add tests for success, forbidden, not found, and unauthenticated cases

### Phase 3: Add updated_ts Column to user_roles Table

#### Step 3.1: Update Existing Migration and Seed Data ✅
- Modify `config/database/migrations/001_create_user_roles.sql` to add `updated_ts INTEGER NOT NULL` column after `created_ts`
- Follow the exact pattern from sites table: `created_ts INTEGER NOT NULL, updated_ts INTEGER NOT NULL,`
- Update `config/database/seeds/001_default_roles.sql` to include `updated_ts` values in INSERT statements
- Set `updated_ts` values to match `created_ts` values for seed data
- Note: No changes needed for rollback file since it just drops the table

#### Step 3.2: Update UserRole Model ✅
- Add `updated_ts: i64` field to `UserRole` struct in `src/models/user_role.rs`
- Update all existing queries that select from user_roles to include `updated_ts`
- Verify all existing tests still pass after model change

#### Step 3.3: Update roles Resolver to Expose updatedTs ✅
- Update `RoleListing` response type in `src/graphql/resolvers/roles.rs` to include `updated_ts` field
- Add `#[graphql(name = "updatedTs")]` annotation following existing patterns
- Update resolver implementation to return the new field
- Update tests to verify `updatedTs` is included in response

#### Step 3.4: Update role Resolver to Expose updatedTs ✅
- Update `RoleResponse` type in `src/graphql/resolvers/role.rs` to include `updated_ts` field
- Add `#[graphql(name = "updatedTs")]` annotation following existing patterns
- Update resolver implementation to return the new field
- Update tests to verify `updatedTs` is included in response

#### Step 3.5: Create Update Query ✅
- Create `src/queries/user_roles/update_role.rs`
- Support partial updates (PATCH semantics)
- Handle permissions array conversion to JSON
- Update `updated_ts` timestamp (following `UpdateSiteQuery` pattern exactly)

#### Step 3.6: Add UserRoleService Method ✅
- Add `update_role(role_id: i64, update_data: UpdateRoleData)` method to existing `src/services/user_role_service.rs`
- Validate all permissions in the input array are valid
- Return `Result<UserRole, RoleError>`

#### Step 3.7: Add Orchestrator Method ✅
- Add `update_role_with_permission_check(pool, session_context, role_id: i64, update_data: UpdateRoleData)` to RoleOrchestrator
- Follow `SiteOrchestrator` patterns exactly:
  - Authentication: Check `session_context.user_id()` and return `AuthenticationError` if None
  - Authorization: Fetch user via `GetUserByIdQuery::run()`, check `can_manage_roles` permission
  - Return `AuthorizationError("Forbidden".to_string())` for unauthorized users
  - Business logic: Call service method and handle not found case
- Validate `can_manage_roles` permission
- Call UserRoleService method
- Add tests following `SiteOrchestrator` patterns for success, forbidden, not found, and unauthenticated cases

#### Step 3.8: Create Resolver ✅
- Create `src/graphql/resolvers/update_role.rs`
- Define `UpdateRoleResponse` type following `UpdateSiteResponse` pattern
- Define `UpdateRoleData` input struct with `permissions: Option<Vec<String>>`
- Implement `UpdateRoleResolver` with `updateRole` mutation
- Add `UpdateRoleResolver` to `Mutation` struct in `src/graphql/schema.rs`
- Handle partial updates with proper null handling
- Add comprehensive tests

### Phase 4: addRole Mutation Resolver
**Permission requirement**: `can_manage_roles`

#### Step 4.1: Create Create Query ✅
- Create `src/queries/user_roles/create_role.rs`
- Handle permissions array conversion to JSON
- Validate role name uniqueness
- Set initial timestamps (both `created_ts` and `updated_ts`)

#### Step 4.2: Add UserRoleService Method ✅
- Add `create_role(create_data: CreateRoleData)` method to existing `src/services/user_role_service.rs`
- Validate all permissions are valid
- Validate role name format and uniqueness
- Return `Result<UserRole, RoleError>`

#### Step 4.3: Add Orchestrator Method ✅
- Add `create_role_with_permission_check(pool, session_context, create_data: CreateRoleData)` to RoleOrchestrator
- Follow `SiteOrchestrator` patterns exactly:
  - Authentication: Check `session_context.user_id()` and return `AuthenticationError` if None
  - Authorization: Fetch user via `GetUserByIdQuery::run()`, check `can_manage_roles` permission
  - Return `AuthorizationError("Forbidden".to_string())` for unauthorized users
  - Business logic: Call service method and handle validation errors
- Validate `can_manage_roles` permission
- Call UserRoleService method
- Add tests following `SiteOrchestrator` patterns for success, forbidden, validation, and unauthenticated cases

#### Step 4.4: Create Resolver ✅
- Create `src/graphql/resolvers/add_role.rs`
- Define `AddRoleResponse` type following `AddSiteResponse` pattern
- Define `AddRoleData` input struct with `permissions: Vec<String>`
- Follow `addSite` pattern for input structure (no envelope)
- Implement `AddRoleResolver` with `addRole` mutation
- Add `AddRoleResolver` to `Mutation` struct in `src/graphql/schema.rs`
- Add comprehensive tests

### Phase 5: removeRole Mutation Resolver
**Permission requirement**: `can_manage_roles`

#### Step 5.1: Create Delete Query
- Create `src/queries/user_roles/delete_role.rs`
- Check if any users are using the role before deletion
- Perform cascade-safe deletion

#### Step 5.2: Add UserRoleService Method
- Add `delete_role(role_id: i64)` method to existing `src/services/user_role_service.rs`
- Check for role usage by users
- Return `Result<(), RoleError>`

#### Step 5.3: Add Orchestrator Method
- Add `delete_role_with_permission_check(pool, session_context, role_id: i64)` to RoleOrchestrator
- Follow `SiteOrchestrator` patterns exactly:
  - Authentication: Check `session_context.user_id()` and return `AuthenticationError` if None
  - Authorization: Fetch user via `GetUserByIdQuery::run()`, check `can_manage_roles` permission
  - Return `AuthorizationError("Forbidden".to_string())` for unauthorized users
  - Business logic: Call service method and handle not found/in use cases
- Validate `can_manage_roles` permission
- Call UserRoleService method
- Add tests following `SiteOrchestrator` patterns for success, forbidden, not found, role in use, and unauthenticated cases

#### Step 5.4: Create Resolver
- Create `src/graphql/resolvers/remove_role.rs`
- Define `RemoveRoleResponse` type with success boolean
- Implement `RemoveRoleResolver` with `removeRole` mutation
- Add `RemoveRoleResolver` to `Mutation` struct in `src/graphql/schema.rs`
- Add tests for success, forbidden, role in use, not found, and unauthenticated cases

### Phase 6: Update README.md

#### Step 6.1: Update GraphQL API Table
- Add new queries to the README.md GraphQL API table:
  - `roles` - List all roles (requires can_manage_roles OR can_edit_user_role)
  - `role` - Get single role by ID (requires can_manage_roles)
- Add new mutations to the README.md GraphQL API table:
  - `addRole` - Create new role (requires can_manage_roles)
  - `updateRole` - Update existing role (requires can_manage_roles)
  - `removeRole` - Delete role (requires can_manage_roles)

## Error Handling

### RoleError Enum
Create comprehensive error enum in `src/orchestrators/role_orchestrator.rs` (following `SiteError` pattern in `src/services/site_service.rs`):
- `DatabaseError(sqlx::Error)`
- `AuthenticationError(String)`
- `AuthorizationError(String)`
- `RoleNotFound(i64)`
- `RoleNameAlreadyExists(String)`
- `RoleInUse(i64)` - for delete operations
- `ValidationError(String)`
- `InvalidPermission(String)` - for invalid permission strings

Implement `Display`, `Error`, and `From<sqlx::Error>` traits following `SiteError` pattern.

### Permission Validation
- All permission arrays must be validated against `ROLE_PERMISSIONS` in `src/models/user_role.rs`
- Use `is_valid_role_permission()` function for validation
- Return descriptive errors for invalid permissions

## Testing Strategy

### Unit Tests
- Test all UserRoleService methods with various inputs
- Test permission validation logic
- Test error handling paths

### Integration Tests
- Test complete resolver workflows
- Test permission enforcement
- Test authentication requirements
- Test database operations

### Test Data Setup
- Create helper functions for setting up test roles with different permissions
- Use existing patterns from site and user resolver tests
- Ensure proper cleanup between tests

## Implementation Notes

### Orchestrator Patterns
All orchestrator methods must follow the exact patterns established by `SiteOrchestrator`:

#### Method Signature Pattern
```rust
pub async fn method_name_with_permission_check(
  pool: &SqlitePool,
  session_context: SessionContext,
  // ... other parameters
) -> Result<ReturnType, RoleError>
```

#### Implementation Pattern
1. **Authentication**: Check `session_context.user_id()` and return `AuthenticationError` if None
2. **Authorization**: 
   - Fetch user via `GetUserByIdQuery::run(pool, user_id)?`
   - Handle user not found with `AuthenticationError("User not found".to_string())`
   - Check permissions via `UserRoleService::check_user_permission(pool, &user, "permission").await?`
   - Return `AuthorizationError("Forbidden".to_string())` if not authorized
3. **Business Logic**: Call service methods and convert errors appropriately
4. **Error Handling**: Map service errors to orchestrator errors using `From` traits

#### Test Pattern
- Use helper functions `create_test_user()` and `create_test_role()`
- Use `SessionContext` with `DpsAuthSessionPayload` for authentication
- Test all cases: success, unauthenticated, nonexistent user, forbidden, not found, validation errors

### Permission Handling
- All resolvers must return `permissions` as `Vec<String>` in GraphQL responses
- Input mutations accept `permissions` as `Vec<String>` and convert to JSON internally
- Use existing `UserRole::permissions()` method for deserialization

### Naming Conventions
- Follow existing patterns: `RolesResolver`, `RoleResolver`, etc.
- GraphQL names: `roles`, `role`, `addRole`, `updateRole`, `removeRole`
- File names: `roles.rs`, `role.rs`, `add_role.rs`, etc.

### Database Operations
- Use existing query patterns
- Ensure proper transaction handling for complex operations
- Update `updated_ts` timestamps for all update operations
- All user_roles queries must include both `created_ts` and `updated_ts` fields

### Security Considerations
- Never expose `permissions_json` directly in GraphQL responses
- Validate all permission inputs against the whitelist
- Ensure proper permission checks before any database operations
- Log security-relevant events (role creation, deletion, permission changes)

## Files to Create/Modify

### New Files
- `src/orchestrators/role_orchestrator.rs`
- `src/graphql/resolvers/roles.rs`
- `src/graphql/resolvers/role.rs`
- `src/graphql/resolvers/add_role.rs`
- `src/graphql/resolvers/update_role.rs`
- `src/graphql/resolvers/remove_role.rs`
- `src/queries/user_roles/create_role.rs`
- `src/queries/user_roles/update_role.rs`
- `src/queries/user_roles/delete_role.rs`

### Modified Files
- `config/database/migrations/001_create_user_roles.sql`
- `config/database/seeds/001_default_roles.sql`
- `src/models/user_role.rs`
- `src/queries/user_roles/get_all_roles.rs`
- `src/queries/user_roles/get_role_by_id.rs`
- `src/queries/user_roles/get_role_by_name.rs`
- `src/queries/user_roles/get_default_user_role.rs`
- `src/graphql/schema.rs`
- `src/services/user_role_service.rs`
- `src/orchestrators/mod.rs`
- `src/graphql/resolvers/mod.rs`
- `src/queries/user_roles/mod.rs`
- `README.md`

This plan ensures a systematic, secure, and well-tested implementation of role management functionality following all established patterns in the codebase.