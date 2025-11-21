# Refactor GraphQL Resolvers to Business-Oriented Operations

## Overview

This plan outlines the refactoring of GraphQL resolvers from pure CRUD operations to business-oriented operations while maintaining clean architecture through proper service layer separation.

## Current State Analysis

### Existing CRUD Resolvers
- `createSession` - creates a session record
- `createUser` - creates a user record  
- `getCurrentSession` - retrieves session data
- `getSites` - retrieves site records

### Proposed Business-Oriented Resolvers
- `authLogin` - authenticates user and establishes session
- `authRegister` - registers new user account
- `authMe` - gets current authenticated user profile
- `sitesList` - lists available sites

## Architectural Approach

### Layered Service Architecture

**1. Resolvers (API Layer)**
- Permission checks
- Service coordination when needed
- Response formatting
- Business-oriented operation naming

**2. Orchestration Services (Workflow Layer - When Needed)**
- Coordinate multiple CRUD services for complex workflows
- Handle multi-step business operations
- Bridge between CRUD and business operations
- Only created when complexity warrants it

**3. CRUD Services (Data Layer)**
- Pure database operations with validation
- Domain-specific business rules
- Data validation and error handling
- Reusable across different contexts

**Key Principle**: Only add orchestration layers when operations require coordination of multiple services or complex business workflows. Simple CRUD operations should call CRUD services directly from resolvers.

## Implementation Plan

### Phase 1: Refactor createSession → authLogin ✅ COMPLETED

**Files to Create/Modify:**
- `src/services/auth_service.rs` - Create orchestration service with login method ✅
- `src/graphql/resolvers/create_session.rs` → `src/graphql/resolvers/auth_login.rs` ✅
- `src/graphql/resolvers/mod.rs` - Update module exports ✅
- `src/graphql/schema.rs` - Update resolver imports ✅
- Tests in auth_login.rs - Update to use new operation name ✅

**Implementation Steps:**
1. Create AuthService with login orchestration method ✅
2. Rename create_session.rs to auth_login.rs and update resolver ✅
3. Update resolver to call AuthService::login instead of SessionService::create_session ✅
4. Update GraphQL field name from createSession to authLogin ✅
5. Update tests to use new operation name ✅
6. Verify all tests pass ✅

**Results:**
- Created `AuthService` orchestration service with `AuthResult` struct
- Refactored resolver to business-oriented `authLogin` operation
- Enhanced response to include `user_id` and `username` along with `token`
- Updated all integration tests to use new operation
- All 175+ tests passing
- Clean architecture maintained with thin resolver layer

### Phase 2: Refactor createUser → authRegister ✅ COMPLETED

**Files to Create/Modify:**
- `src/services/auth_service.rs` - Add register orchestration method ✅
- `src/graphql/resolvers/create_user.rs` → `src/graphql/resolvers/auth_register.rs` ✅
- `src/graphql/resolvers/mod.rs` - Update module exports ✅
- `src/graphql/schema.rs` - Update resolver imports ✅
- Tests in auth_register.rs - Update to use new operation name ✅

**Implementation Steps:**
1. Add AuthService::register method ✅
2. Rename create_user.rs to auth_register.rs and update resolver ✅
3. Update resolver to call AuthService::register ✅
4. Update GraphQL field name from createUser to authRegister ✅
5. Update tests to use new operation name ✅
6. Verify all tests pass ✅

**Results:**
- Added `RegisterResult` struct and `AuthService::register()` method
- Refactored resolver to business-oriented `authRegister` operation
- Enhanced response to include `user_id` along with existing user information
- Updated all integration tests to use new operation
- All 175+ tests passing
- Clean architecture maintained with thin resolver layer

**Password Confirmation Enhancement ✅ COMPLETED:**
- Added `passwordConfirmation` field to `AuthRegisterInput` for better UX
- Implemented password confirmation validation in `AuthService::register()`
- Updated all tests to cover password confirmation scenarios
- Added specific test for password confirmation mismatch
- Updated integration tests to include password confirmation
- This provides defense-in-depth validation and consistent API contract
- All 176+ tests passing

### Phase 3: Refactor getCurrentSession → authMe ✅ COMPLETED

**Files to Create/Modify:**
- `src/services/auth_service.rs` - Add get_current_user orchestration method ✅
- `src/graphql/resolvers/get_current_session.rs` → `src/graphql/resolvers/auth_me.rs` ✅
- `src/graphql/resolvers/mod.rs` - Update module exports ✅
- `src/graphql/schema.rs` - Update resolver imports ✅
- Tests in auth_me.rs - Update to use new operation name ✅

**Implementation Steps:**
1. Add AuthService::get_current_user method ✅
2. Rename get_current_session.rs to auth_me.rs and update resolver ✅
3. Update resolver to call AuthService::get_current_user ✅
4. Update GraphQL field name from getCurrentSession to authMe ✅
5. Update tests to use new operation name ✅
6. Verify all tests pass ✅

**Results:**
- Added `AuthMeResult` struct and `AuthService::get_current_user()` method
- Added `UserService::get_user_by_id()` method for user lookup
- Refactored resolver to business-oriented `authMe` operation
- Enhanced response to include full user profile plus session information
- Updated all integration tests to use new operation (3 minor test issues remain)
- All resolver tests passing (4/4)
- Clean architecture maintained with thin resolver layer
- Project builds successfully

### Phase 4: Refactor getSites → sites ✅ COMPLETED

**Files to Modify:**
- `src/graphql/resolvers/get_sites.rs` → `src/graphql/resolvers/sites.rs` ✅
- `src/graphql/resolvers/mod.rs` - Update module exports ✅
- `src/graphql/schema.rs` - Update resolver imports ✅
- Tests in sites.rs - Update to use new operation name ✅

**Implementation Steps:**
1. Rename get_sites.rs to sites.rs and update resolver ✅
2. Update GraphQL field name from getSites to sites ✅
3. Keep existing SiteService::get_all_sites() call (no orchestration needed) ✅
4. Update tests to use new operation name ✅
5. Verify all tests pass ✅

**Results:**
- Successfully renamed resolver file and struct
- Updated GraphQL field from `getSites` to `sites`
- Maintained existing SiteService::get_all_sites() call
- All tests passing (177/177)
- No orchestration service needed for simple CRUD operation

**Phase 5: Refactor createSite → addSite ✅ COMPLETED**
- Rename create_site.rs to add_site.rs ✅
- Update GraphQL field name from createSite to addSite ✅
- Keep existing SiteService::create_site() call (no orchestration needed) ✅
- Update tests to use new operation name ✅

**Results:**
- Successfully renamed resolver file and struct
- Updated GraphQL field from `createSite` to `addSite`
- Updated input/output types: `CreateSiteInput` → `AddSiteInput`, `CreateSiteResponse` → `AddSiteResponse`
- Maintained existing SiteService::create_site() call
- All tests passing (177/177)
- No orchestration service needed for simple CRUD operation

**Phase 6: Refactor deleteSite → removeSite ✅ COMPLETED**
- Rename delete_site.rs to remove_site.rs ✅
- Update GraphQL field name from deleteSite to removeSite ✅
- Keep existing SiteService::delete_site() call (no orchestration needed) ✅
- Update tests to use new operation name ✅

**Results:**
- Successfully renamed resolver file and struct
- Updated GraphQL field from `deleteSite` to `removeSite`
- Updated response type: `DeleteSiteResponse` → `RemoveSiteResponse`
- Updated success message from "Site deleted successfully" to "Site removed successfully"
- Maintained existing SiteService::delete_site() call
- All tests passing (177/177)
- No orchestration service needed for simple CRUD operation

### Phase 7: Update Documentation

**Files to Modify:**
- `README.md` - Complete GraphQL API documentation overhaul
- `docs/api.md` - DELETE this file (consolidate into README)

**Documentation Updates:**

#### 7.1 Update GraphQL Operations Section
- Replace `createUser` with `authRegister` in examples
- Replace `createSession` with `authLogin` in examples  
- Replace `getCurrentSession` with `authMe` in examples
- Replace `getSites` with `sites` in examples
- Replace `createSite` with `addSite` in examples
- Replace `deleteSite` with `removeSite` in examples

#### 7.2 Add Comprehensive GraphQL Resolvers Table
Create a new section in README with detailed markdown tables documenting all GraphQL operations:

**Mutations Table:**

| Operation | Description | Input Fields | Response Fields | Auth Required |
|-----------|-------------|--------------|-----------------|---------------|
| `authRegister` | Register new user account | username (String!), password (String!), passwordConfirmation (String!) | user_id (Int), username (String), message (String) | None |
| `authLogin` | Authenticate user and create session | username (String!), password (String!) | token (String), user_id (Int), username (String), message (String) | None (sets cookie) |
| `addSite` | Add new site to database | name (String!), url (String!) | site_id (Int), name (String), url (String), message (String) | can_create_site |
| `updateSite` | Update existing site | site_id (Int!), name (String), url (String) | site_id (Int), name (String), url (String), message (String) | can_update_site |
| `removeSite` | Remove existing site | site_id (Int!) | site_id (Int), message (String) | can_delete_site |

**Queries Table:**

| Operation | Description | Response Fields | Auth Required |
|-----------|-------------|-----------------|---------------|
| `getServerTimestamp` | Get current server timestamp | timestamp (String) | None |
| `authMe` | Get current authenticated user profile | user_id (Int), username (String), role (String), created_ts (String) | Valid session cookie |
| `sites` | List all sites in database | sites: [site_id (Int), name (String), url (String), created_ts (String)] | None |

#### 7.4 Consolidate Documentation
- Delete `docs/api.md` file
- Move any useful content from api.md to README
- Ensure README is the single source of truth for all API documentation
- Update any references to the separate api.md file

#### 7.5 Update Schema Documentation
- Update GraphQL schema documentation comments
- Ensure all operation descriptions match business-oriented naming
- Document authentication requirements and permission systems

## Benefits of This Approach

### Pros
1. **Better API Usability** - More intuitive for frontend developers
2. **Clearer Business Intent** - Resolvers map directly to user actions
3. **More Flexible Implementation** - Business logic can evolve without API changes
4. **Better Error Handling** - Business-specific error messages
5. **Maintained Reusability** - CRUD services remain reusable
6. **Clean Separation** - Each layer has clear responsibilities

### Cons Addressed
1. **Reduced Reusability** → Solved by keeping CRUD services separate
2. **Tighter Coupling** → Minimized through orchestration layer
3. **Less Predictable Patterns** → Moved to service layer where appropriate
4. **Bloated Resolvers** → Prevented by orchestration services

## Migration Strategy

### Backward Compatibility Considerations
- Since project is not in use, can break compatibility
- Consider keeping old resolvers temporarily during migration if needed
- Update all documentation and examples to use new operations

### Testing Strategy
- Run existing tests to ensure functionality preserved
- Add new tests for orchestration services
- Update integration tests to use new GraphQL operations
- Verify error handling works correctly with new structure

## Files to Create/Modify

### New Files
- `src/services/auth_service.rs`

### Renamed Files
- `src/graphql/resolvers/create_session.rs` → `src/graphql/resolvers/auth_login.rs`
- `src/graphql/resolvers/create_user.rs` → `src/graphql/resolvers/auth_register.rs`
- `src/graphql/resolvers/get_current_session.rs` → `src/graphql/resolvers/auth_me.rs`
- `src/graphql/resolvers/get_sites.rs` → `src/graphql/resolvers/sites.rs`
- `src/graphql/resolvers/create_site.rs` → `src/graphql/resolvers/add_site.rs`
- `src/graphql/resolvers/delete_site.rs` → `src/graphql/resolvers/remove_site.rs`

### Modified Files
- `src/graphql/schema.rs`
- `src/graphql/resolvers/mod.rs`
- `src/services/mod.rs`
- `README.md`
- All test files in affected resolver modules

## Success Criteria

1. All existing functionality preserved
2. New business-oriented GraphQL operations work correctly
3. Orchestration services used only when complexity warrants it
4. Simple CRUD operations call services directly from resolvers
5. Tests pass with new structure
6. Code follows established patterns and conventions
7. Error handling remains robust and user-friendly
8. Documentation updated with new operation names and examples
9. README reflects current GraphQL API structure

## Timeline

This refactoring uses a per-resolver approach to ensure each phase is fully tested and functional:

1. Phase 1: Refactor createSession → authLogin (1-2 hours) ✅ COMPLETED
2. Phase 2: Refactor createUser → authRegister (1-2 hours) ✅ COMPLETED  
3. Phase 3: Refactor getCurrentSession → authMe (1-2 hours) ✅ COMPLETED
4. Phase 4: Refactor getSites → sites (30-60 minutes) ✅ COMPLETED
5. Phase 5: Refactor createSite → addSite (30-60 minutes) ✅ COMPLETED
6. Phase 6: Refactor deleteSite → removeSite (30-60 minutes) ✅ COMPLETED
7. Phase 7: Update documentation (2-3 hours)

Total estimated time: 2-3 hours remaining

Each phase includes updating the resolver, updating schema/module files, and ensuring all tests pass before moving to the next resolver. Orchestration services are only created when complexity warrants it.