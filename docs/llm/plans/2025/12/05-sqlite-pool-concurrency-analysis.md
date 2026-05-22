# SQLite Pool Usage Concurrency Analysis

## Date Created
2025-12-05

## Issue Summary
The recent test failures revealed a critical concurrency issue where database operations expecting to see each other's changes were failing when using connection pools with multiple connections. This analysis identifies potential production code vulnerabilities and provides recommendations for mitigation.

## Root Cause Analysis

### The Problem
Connection pools with multiple connections can execute operations on different database connections that don't immediately see each other's uncommitted changes. In SQLite, this is particularly problematic because:

1. **Default Isolation**: Each connection operates independently until transactions are committed
2. **No Cross-Connection Visibility**: Changes made on one connection aren't visible to other connections until commit
3. **Foreign Key Constraints**: Referential integrity checks fail when child operations can't see parent records created on other connections

### Test Evidence
The failing tests demonstrated this perfectly:
- Tests create roles, then immediately create users referencing those roles
- With pool size > 1, these operations can execute on different connections
- Result: "FOREIGN KEY constraint failed" errors

## Production Code Vulnerability Analysis

### 🔴 HIGH RISK: Multi-Operation Patterns

#### 1. Authentication Flow (`src/services/auth_service.rs`)
**Location**: `AuthService::login()` method (lines 62-86)
**Pattern**: 
```rust
let user = UserService::get_user_by_name(pool, username).await?;  // Connection A
let session_token = SessionService::create_session(pool, username, password, session_secret).await?;  // Connection B
```
**Risk**: User lookup and session creation run on different connections
**Impact**: Low - Session creation doesn't depend on user lookup results

#### 2. User Registration Flow (`src/services/auth_service.rs`)
**Location**: `AuthService::register()` method (lines 107-131)
**Pattern**:
```rust
let user = UserService::create_user(pool, username, password).await?;  // Creates user
// Returns user data immediately
```
**Risk**: Low - Single operation, no dependent queries

#### 3. Orchestrator Permission Checks
**Location**: Multiple orchestrators (user, site, auth)
**Pattern**:
```rust
// Example from src/orchestrators/site_orchestrator.rs:23-30
let user = GetUserByIdQuery::run(pool, user_id).await?;  // Connection A
let allowed = UserRoleService::check_user_permission(pool, &user, "can_create_site").await?;  // Connection B
```
**Risk**: Medium - Permission check depends on user data, but user is passed as parameter

#### 4. User Creation with Role Assignment
**Location**: Multiple test files and potentially production code
**Pattern**:
```rust
let role_id = create_role(pool, "admin", permissions).await;  // Connection A
let user = create_user(pool, "admin", role_id).await;  // Connection B - FOREIGN KEY RISK
```
**Risk**: HIGH - User creation depends on role existence

### 🟡 MEDIUM RISK: Test Infrastructure

#### 1. Test Database Setup
**Location**: All test files using `create_test_database()`
**Pattern**: Tests create roles then users in sequence
**Current Mitigation**: Fixed by using pool size 1 for tests
**Production Impact**: None (tests only)

### 🟢 LOW RISK: Single-Operation Patterns

#### 1. Simple CRUD Operations
**Location**: Most service methods
**Pattern**: Single database operation per method call
**Risk**: Low - No dependent operations within same method

## Current Production Safeguards

### 1. Configuration Defaults
- **Default Pool Size**: 10 connections (from README examples)
- **Test Pool Size**: 1 connection (properly configured)
- **Production Risk**: HIGH with default configuration

### 2. Transaction Usage
- **Limited Transaction Usage**: Most operations don't use explicit transactions
- **Auto-commit Mode**: Each statement commits immediately
- **Impact**: Reduces but doesn't eliminate race conditions

## Recommended Solutions

### Phase 1: Immediate Risk Mitigation

#### 1.1 Update Production Default Pool Size
**File**: `README.md` and documentation
**Change**: Reduce default pool size from 10 to 1-3 connections
**Rationale**: Minimize concurrency issues while maintaining some performance benefits

#### 1.2 Add Transaction Wrappers
**Files**: All orchestrator methods
**Implementation**: Wrap multi-step operations in explicit transactions
```rust
pub async fn create_site_with_permission_check(
    pool: &SqlitePool,
    session_context: SessionContext,
    create_data: CreateSiteData,
) -> Result<crate::models::Site, SiteError> {
    let mut tx = pool.begin().await?;
    
    // All operations use the same transaction
    let user = GetUserByIdQuery::run(&mut *tx, user_id).await?;
    let allowed = UserRoleService::check_user_permission(&mut *tx, &user, "can_create_site").await?;
    
    if !allowed {
        tx.rollback().await?;
        return Err(SiteError::AuthorizationError("Forbidden".to_string()));
    }
    
    let site = SiteService::create_site(&mut *tx, create_data).await?;
    tx.commit().await?;
    Ok(site)
}
```

#### 1.3 Update Query Interfaces
**Files**: All query files in `src/queries/`
**Change**: Support both `SqlitePool` and `Transaction<'_, Sqlite>` types
**Implementation**: Use trait bounds or generic types

### Phase 2: Architectural Improvements

#### 2.1 Implement Unit of Work Pattern
**File**: New `src/database/unit_of_work.rs`
**Purpose**: Ensure related operations use same connection/transaction
**Interface**:
```rust
pub struct UnitOfWork<'a> {
    transaction: Transaction<'a, Sqlite>,
}

impl<'a> UnitOfWork<'a> {
    pub async fn execute<F, R>(&mut self, operation: F) -> Result<R, DatabaseError>
    where F: FnOnce(&mut Transaction<'_, Sqlite>) -> Pin<Box<dyn Future<Output = Result<R, DatabaseError>> + Send>>
}
```

#### 2.2 Update Service Layer
**Files**: All service files
**Change**: Accept `&mut Transaction` instead of `&SqlitePool` where appropriate
**Benefits**: Guaranteed connection consistency

#### 2.3 Add Connection Pool Monitoring
**File**: New `src/database/pool_monitor.rs`
**Purpose**: Track pool usage and detect potential contention
**Metrics**: Connection wait times, pool exhaustion, concurrent operations

### Phase 3: Advanced Safeguards

#### 3.1 Implement Database-Level Constraints
**Files**: Migration files
**Add**: DEFERRED foreign key constraints where appropriate
**Example**:
```sql
PRAGMA defer_foreign_keys = ON;
```

#### 3.2 Add Retry Logic
**Files**: Service layer
**Purpose**: Handle transient concurrency errors
**Implementation**: Exponential backoff for foreign key failures

#### 3.3 Connection Affinity
**File**: New `src/database/affinity.rs`
**Purpose**: Route related operations to same pool connection
**Strategy**: Hash-based connection routing for related operations

## Implementation Priority

### Immediate (This Sprint)
1. ✅ **Fix test pool sizes** (COMPLETED)
2. 🔄 **Update production default pool size** to 3
3. 🔄 **Add transaction wrappers to high-risk orchestrators**

### Short Term (Next Sprint)
1. **Implement Unit of Work pattern**
2. **Update query interfaces for transaction support**
3. **Add connection pool monitoring**

### Medium Term (Next Quarter)
1. **Database-level constraint optimization**
2. **Retry logic implementation**
3. **Connection affinity system**

## Risk Assessment Matrix

| Component | Current Risk | Post-Mitigation Risk | Priority |
|-----------|---------------|---------------------|----------|
| Authentication Flow | Low | Low | Low |
| User Registration | Low | Low | Low |
| Permission Checks | Medium | Low | High |
| Role→User Creation | High | Low | Critical |
| Test Infrastructure | Low | Low | Low |

## Testing Strategy

### 1. Concurrency Tests
```rust
#[tokio::test]
async fn test_concurrent_role_user_creation() {
    let (pool, _temp_file) = create_test_database_with_pool_size(10).await;
    
    let role_id = create_role(&pool, "admin", &["can_create_user"]).await;
    
    // Simulate concurrent user creation
    let user1_fut = create_user(&pool, "user1", role_id);
    let user2_fut = create_user(&pool, "user2", role_id);
    
    let (user1, user2) = tokio::join!(user1_fut, user2_fut);
    assert!(user1.is_ok());
    assert!(user2.is_ok());
}
```

### 2. Transaction Isolation Tests
Verify that operations within transactions see consistent state.

### 3. Pool Contention Tests
Test behavior under high load with maximum pool size.

## Monitoring and Alerting

### 1. Database Metrics
- Connection pool utilization
- Transaction rollback rates
- Foreign key constraint failures
- Query execution times

### 2. Application Metrics
- Request success/failure rates
- Authentication success rates
- Permission check failures

### 3. Alert Thresholds
- Pool utilization > 80%
- Foreign key failure rate > 1%
- Transaction rollback rate > 5%

## Conclusion

The pool size issue revealed significant architectural vulnerabilities in the production codebase. While the immediate test fixes resolved the symptom, the underlying concurrency risks remain in production. Implementing the recommended solutions will:

1. **Eliminate race conditions** through proper transaction usage
2. **Maintain performance** with optimized pool sizing
3. **Provide observability** through monitoring and metrics
4. **Ensure data consistency** across all operations

The phased approach allows for immediate risk reduction while building toward a robust, concurrent-safe architecture.