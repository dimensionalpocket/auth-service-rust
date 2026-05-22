# Plan: Refactor Query Layer Tests - Raw INSERT Statement Replacement

**Date:** 2025-12-14@14:30  
**Status:** Analysis Complete  
**Priority:** Optional (Query layer tests marked as acceptable in original plan)

## Executive Summary

Analysis of query layer tests revealed **36 raw INSERT statements** across **12 query test files**. These are unit tests for query objects that legitimately use raw SQL to test the query layer itself. However, for completeness and consistency, they could be refactored to use centralized test utilities.

## Files Requiring Changes

### **High Impact Files** (Multiple INSERT statements):

#### **1. `src/queries/users/create_user.rs`**
- **INSERT Statements:** 7 (lines 76, 109, 163, 192, 200, 227)
- **Test Functions:** `test_create_user_success`, `test_create_user_duplicate_username`, `test_create_user_duplicate_role_id`
- **Replacement Strategy:** 
  - Replace role INSERTs with `create_test_role_model()`
  - Keep user INSERTs as-is (testing the actual query being tested)
- **Impact:** Medium - Tests the core user creation query

#### **2. `src/queries/users/get_all_users_with_roles.rs`**
- **INSERT Statements:** 6 (lines 76, 81, 87, 92, 112, 118)
- **Test Functions:** `test_get_all_users_with_roles_with_data`, `test_get_all_users_with_roles_with_metadata`
- **Replacement Strategy:**
  - Replace role INSERTs with `create_test_role_model()`
  - Replace user INSERTs with `create_test_user_full()` or manual user creation
- **Impact:** Medium - Tests complex JOIN query functionality

#### **3. `src/queries/users/get_user_by_name.rs`**
- **INSERT Statements:** 4 (lines 29, 39, 64, 74)
- **Test Functions:** `test_get_user_by_name_success`, `test_get_user_by_name_not_found`
- **Replacement Strategy:**
  - Replace role INSERTs with `create_test_role_model()`
  - Replace user INSERTs with `create_test_user_full()`
- **Impact:** Low - Tests basic user lookup by name

#### **4. `src/queries/users/delete_user_by_id.rs`**
- **INSERT Statements:** 4 (lines 29, 39, 87, 97)
- **Test Functions:** `test_delete_user_by_id_success`, `test_delete_user_by_id_not_found`
- **Replacement Strategy:**
  - Replace role INSERTs with `create_test_role_model()`
  - Replace user INSERTs with `create_test_user_full()`
- **Impact:** Low - Tests user deletion functionality

### **Medium Impact Files** (2-3 INSERT statements):

#### **5. `src/queries/roles/get_default_role.rs`**
- **INSERT Statements:** 2 (lines 42, 60)
- **Test Functions:** `test_get_default_role_success`, `test_get_default_role_multiple_default_roles`
- **Replacement Strategy:** Replace with `create_test_role_model()` calls
- **Impact:** Low - Tests default role lookup logic

#### **6. `src/queries/users/get_user_by_id.rs`**
- **INSERT Statements:** 2 (lines 29, 39)
- **Test Functions:** `test_get_user_by_id_success`, `test_get_user_by_id_not_found`
- **Replacement Strategy:**
  - Replace role INSERTs with `create_test_role_model()`
  - Replace user INSERTs with `create_test_user_full()`
- **Impact:** Low - Tests user lookup by ID

#### **7. `src/queries/users/get_user_by_uuid.rs`**
- **INSERT Statements:** 2 (lines 29, 39)
- **Test Functions:** `test_get_user_by_uuid_success`, `test_get_user_by_uuid_not_found`
- **Replacement Strategy:**
  - Replace role INSERTs with `create_test_role_model()`
  - Replace user INSERTs with `create_test_user_full()`
- **Impact:** Low - Tests user lookup by UUID

#### **8. `src/queries/users/update_user.rs`**
- **INSERT Statements:** 2 (lines 109, 118)
- **Test Functions:** `test_update_user_success`
- **Replacement Strategy:** Replace role INSERTs with `create_test_role_model()`
- **Impact:** Low - Tests user update functionality

#### **9. `src/queries/users/update_user_password.rs`**
- **INSERT Statements:** 1 (line 66)
- **Test Functions:** `test_update_user_password_success`
- **Replacement Strategy:** Replace role INSERTs with `create_test_role_model()`
- **Impact:** Low - Tests password update functionality

### **Low Impact Files** (1 INSERT statement):

#### **10. `src/queries/roles/get_role_by_name.rs`**
- **INSERT Statements:** 2 (lines 44, 74)
- **Test Functions:** `test_get_role_by_name_success`, `test_get_role_by_name_not_found`
- **Replacement Strategy:** Replace with `create_test_role_model()` calls
- **Impact:** Low - Tests role lookup by name

#### **11. `src/queries/roles/get_role_by_id.rs`**
- **INSERT Statements:** 1 (line 44)
- **Test Functions:** `test_get_role_by_id_success`, `test_get_role_by_id_not_found`
- **Replacement Strategy:** Replace with `create_test_role_model()` calls
- **Impact:** Low - Tests role lookup by ID

#### **12. `src/queries/roles/get_all_roles.rs`**
- **INSERT Statements:** 1 (line 43)
- **Test Functions:** `test_get_all_roles_with_data`
- **Replacement Strategy:** Replace with multiple `create_test_role_model()` calls
- **Impact:** Low - Tests role listing functionality

## Special Cases (Do NOT Replace)

#### **`src/queries/roles/create_role.rs`** - Line 30
- **Type:** This is the actual query implementation, not test setup
- **Status:** **DO NOT REPLACE** - This is the query being tested
- **Reason:** Core functionality of the query object

#### **`src/queries/sites/create_site.rs`** - Line 22
- **Type:** This is the actual query implementation, not test setup
- **Status:** **DO NOT REPLACE** - This is the query being tested
- **Reason:** Core functionality of the query object

## Replacement Strategy by Query Type

### **Role INSERT Statements** (24 total)
**Pattern:** `INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES (...)`
**Replacement:** `create_test_role_model(&pool, "role_name", &["permission1", "permission2"], is_default).await`

**Examples:**
```rust
// BEFORE
sqlx::query("INSERT INTO roles (name, created_ts, updated_ts, is_default, permissions_json) VALUES ('admin', 1234567890, 1234567890, FALSE, '[]')")
  .execute(&pool)
  .await
  .unwrap();

// AFTER
create_test_role_model(&pool, "admin", &[], false).await;
```

### **User INSERT Statements** (10 total)
**Pattern:** `INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES (...)`
**Replacement:** `create_test_user_full(&pool, uuid, name, role_id, password_hash, metadata_json).await`

**Examples:**
```rust
// BEFORE
sqlx::query("INSERT INTO users (uuid, created_ts, updated_ts, name, role_id, password_hash, metadata_json) VALUES ('user1-uuid', 1234567890, 1234567890, 'Alice', 1, 'hash1', NULL)")
  .execute(&pool)
  .await
  .unwrap();

// AFTER
create_test_user_full(&pool, "user1-uuid", "Alice", 1, "hash1", None).await;
```

## Implementation Requirements

### **Import Updates Required**
Each file will need to import test utilities:
```rust
use crate::test_utils::{create_test_database, create_test_role_model, create_test_user_full};
```

### **Test Data Consistency**
- Ensure role IDs match when creating users (use returned role IDs)
- Maintain UUID consistency for user tests
- Preserve metadata JSON where required by tests

### **Complex Test Scenarios**
Some tests require specific data relationships:
- Multiple roles with different default flags
- Users with specific role assignments
- Metadata JSON preservation
- Timestamp ordering verification

## Estimated Effort

- **High Impact Files:** 4-6 hours (4 files)
- **Medium Impact Files:** 2-3 hours (5 files)  
- **Low Impact Files:** 1-2 hours (3 files)
- **Total:** 7-11 hours

## Risk Assessment

### **Benefits:**
1. **Consistency:** All test data setup uses centralized utilities
2. **Maintainability:** Schema changes only require test utility updates
3. **Standardization:** Uniform test patterns across entire codebase

### **Risks:**
1. **Test Complexity:** Some query tests require specific data relationships
2. **Query Layer Focus:** These tests are specifically testing raw SQL queries
3. **Maintenance Overhead:** May make query tests less focused on query logic

## Recommendation

**Status:** OPTIONAL - Not critical for compliance

**Reasoning:** 
- Query layer tests legitimately use raw SQL to test the query layer itself
- Original plan correctly identified these as "acceptable"
- Current state achieves 100% compliance with AGENTS.md guidelines
- Refactoring would be for completeness only, not compliance

**Decision:** 
- **Do NOT proceed** unless specifically requested for completeness
- Current refactoring effort successfully meets all project requirements
- Focus should remain on higher-value improvements

## Success Criteria (If Implemented)

1. ✅ All 34 replaceable INSERT statements use test utilities
2. ✅ All query tests maintain original functionality
3. ✅ No regression in test coverage or behavior
4. ✅ Consistent test data creation patterns across query layer
5. ✅ All 457+ tests continue to pass

## Current Status

**Query Layer Tests:** 36 raw INSERT statements identified
- **Replaceable:** 34 (test setup data)
- **Non-replaceable:** 2 (actual query implementations)
- **Files Affected:** 12 out of 12 query test files
- **Priority:** Optional (completeness only)