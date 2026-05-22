# Refactor GraphQL Resolvers to Use Orchestration Layer

## Overview

This plan refactors all GraphQL resolvers to follow the new orchestration layer convention defined in AGENTS.md. Currently, resolvers are directly calling multiple services and handling authentication/authorization logic. According to the new architecture:

- **Resolvers** should be thin wrappers calling at most a single service or orchestrator method
- **Orchestrators** should handle authentication → authorization → business logic patterns
- **Services** should contain core business logic and interact with the database layer

## Current State Analysis

After scanning the codebase, the following resolvers need refactoring:

1. **AuthLoginResolver** - Calls AuthService directly (OK - single service)
2. **AuthRegisterResolver** - Calls AuthService directly (OK - single service) 
3. **AuthMeResolver** - Calls AuthService directly (OK - single service)
4. **AuthLogoutResolver** - No service calls (OK - simple operation)
5. **AuthChangePasswordResolver** - Calls UserService directly, but handles authentication logic (NEEDS orchestrator)
6. **AddSiteResolver** - Calls multiple services: GetUserByIdQuery, UserRoleService, SiteService (NEEDS orchestrator)
7. **RemoveSiteResolver** - Calls multiple services: GetUserByIdQuery, UserRoleService, SiteService (NEEDS orchestrator)
8. **UpdateSiteResolver** - Calls multiple services: GetUserByIdQuery, UserRoleService, SiteService (NEEDS orchestrator)
9. **SitesResolver** - Calls SiteService directly (OK - single service)
10. **GetServerTimestampResolver** - Calls ServerService directly (OK - single service)

## Phases

Each phase is self-contained and can be implemented independently.

### Phase 1: AuthChangePasswordResolver ✅ COMPLETED

**Files created:**
- ✅ `src/orchestrators/mod.rs` - New module file
- ✅ `src/orchestrators/auth_orchestrator.rs` - New orchestrator for authentication operations
- ✅ `src/orchestrators/site_orchestrator.rs` - New orchestrator for site operations (placeholder)

**Files modified:**
- ✅ `src/graphql/resolvers/auth_change_password.rs` - Refactored to use orchestrator
- ✅ `src/lib.rs` - Added orchestrators module
- ✅ `src/services/user_service.rs` - Added AuthenticationError variant to UserError enum

**Implementation completed:**

1. ✅ Created `src/orchestrators/mod.rs` with module exports
2. ✅ Created `src/orchestrators/auth_orchestrator.rs` with authentication → authorization → business logic pattern
3. ✅ Updated `src/graphql/resolvers/auth_change_password.rs`:
   - Removed SessionContext handling and authentication logic
   - Removed GetUserByIdQuery import
   - Added AuthOrchestrator import
   - Calls AuthOrchestrator::change_authenticated_user_password instead of UserService::update_password
   - Added proper AuthenticationError handling in GraphQL error mapping

**Testing completed:**
- ✅ All 5 auth change password tests pass
- ✅ Code passes clippy and rustfmt checks
- ✅ Maintains backward compatibility and existing functionality

### Phase 2: AddSiteResolver ✅ COMPLETED

**Files created:**
- ✅ `src/orchestrators/site_orchestrator.rs` - New orchestrator for site operations

**Files modified:**
- ✅ `src/services/site_service.rs` - Added AuthenticationError and AuthorizationError variants to SiteError enum
- ✅ `src/graphql/resolvers/add_site.rs` - Refactored to use orchestrator

**Implementation completed:**

1. ✅ Created `src/orchestrators/site_orchestrator.rs` with `create_site_with_permission_check` method:
   - Authentication: Check if user is authenticated via SessionContext
   - Authorization: Get user and check "can_create_site" permission
   - Business logic: Delegate to SiteService::create_site

2. ✅ Updated `src/graphql/resolvers/add_site.rs`:
   - Removed SessionContext handling and authentication logic
   - Removed GetUserByIdQuery and UserRoleService imports
   - Added SiteOrchestrator import
   - Calls SiteOrchestrator::create_site_with_permission_check instead of handling auth/authz directly
   - Added proper error handling for new AuthenticationError and AuthorizationError variants

3. ✅ Updated `src/services/site_service.rs`:
   - Added AuthenticationError(String) and AuthorizationError(String) variants to SiteError enum
   - Updated Display implementation to handle new error types

**Testing completed:**
- ✅ All 5 add_site tests pass (success, forbidden, unauthenticated, duplicate slug, invalid slug)
- ✅ Code passes clippy and rustfmt checks
- ✅ Maintains backward compatibility and existing functionality

### Phase 3: RemoveSiteResolver ✅ COMPLETED

**Files modified:**
- ✅ `src/orchestrators/site_orchestrator.rs` - Added site removal orchestration
- ✅ `src/graphql/resolvers/remove_site.rs` - Refactored to use orchestrator

**Implementation completed:**

1. ✅ Added `remove_site_with_permission_check` method to SiteOrchestrator:
   - Authentication: Check if user is authenticated via SessionContext
   - Authorization: Get user and check "can_delete_site" permission
   - Business logic: Delegate to SiteService::delete_site

2. ✅ Updated `src/graphql/resolvers/remove_site.rs`:
   - Removed SessionContext handling and authentication logic
   - Removed GetUserByIdQuery and UserRoleService imports
   - Added SiteOrchestrator import
   - Calls SiteOrchestrator::remove_site_with_permission_check instead of handling auth/authz directly
   - Added proper error handling for new AuthenticationError and AuthorizationError variants

**Testing completed:**
- ✅ All 4 remove_site tests pass (success, forbidden, unauthenticated, not found)
- ✅ Code passes clippy and rustfmt checks
- ✅ Maintains backward compatibility and existing functionality

### Phase 4: UpdateSiteResolver ✅ COMPLETED

**Files modified:**
- ✅ `src/orchestrators/site_orchestrator.rs` - Added site update orchestration
- ✅ `src/graphql/resolvers/update_site.rs` - Refactored to use orchestrator

**Implementation completed:**

1. ✅ Added `update_site_with_permission_check` method to `src/orchestrators/site_orchestrator.rs`:
   - Authentication: Check if user is authenticated via SessionContext
   - Authorization: Get user and check "can_update_site" permission
   - Business logic: Delegate to SiteService::update_site

2. ✅ Updated `src/graphql/resolvers/update_site.rs`:
   - Removed SessionContext handling and authentication logic
   - Removed GetUserByIdQuery and UserRoleService imports
   - Added SiteOrchestrator import
   - Calls SiteOrchestrator::update_site_with_permission_check instead of handling auth/authz directly
   - Added proper error handling for AuthenticationError and AuthorizationError variants

**Testing completed:**
- ✅ All 13 update_site tests pass (success, forbidden, unauthenticated, not found, invalid slug)
- ✅ All 196 total tests pass
- ✅ Code passes clippy and rustfmt checks
- ✅ Maintains backward compatibility and existing functionality

### Phase 5: Update Error Types and Module Structure ✅ COMPLETED

**Files verified:**
- ✅ `src/services/site_service.rs` - AuthenticationError and AuthorizationError variants already exist from previous phases
- ✅ `src/lib.rs` - orchestrators module already included from previous phases
- ✅ All imports verified and working correctly

**Implementation completed:**

1. ✅ Error types in services already include required variants:
   - `AuthenticationError(String)` for auth failures
   - `AuthorizationError(String)` for permission failures
   - Display implementation already handles these error types

2. ✅ Module structure in `src/lib.rs` already includes:
   - `pub mod orchestrators;` (added in previous phases)

3. ✅ All refactoring verified to work correctly through comprehensive testing.

## Testing Strategy

Each phase includes comprehensive tests:
- Unit tests for orchestrator methods
- Integration tests for resolver methods
- Error path testing for authentication/authorization failures
- Success path testing for valid operations

## Notes

- Resolvers that already call only a single service (AuthLoginResolver, AuthRegisterResolver, AuthMeResolver, SitesResolver, GetServerTimestampResolver) do not need changes
- AuthLogoutResolver is simple cookie manipulation and doesn't need changes
- The orchestration layer consolidates the authentication → authorization → business logic pattern
- Error handling is moved to the orchestrator layer, with resolvers only handling GraphQL-specific error formatting
- TODO: Update README.md mutations/queries table after all phases are complete

## Dependencies

This plan uses existing crates only:
- No new dependencies required
- Removed unnecessary `async-trait` usage from orchestrators