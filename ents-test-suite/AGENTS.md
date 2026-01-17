# Ents Test Suite

The `ents-test-suite` is a comprehensive test suite designed to validate implementations of the `ents` entity framework. This document describes how to use the test suite with different storage backends.

## Overview

The test suite uses the `TransactionProvider` trait from the `ents` crate to run the same tests against different storage engines. Any type that implements `TransactionProvider` can be used directly with the test suite.

## TransactionProvider Trait

The `TransactionProvider` trait (defined in the `ents` crate) provides a standard interface for executing transactional operations:

```rust
pub trait TransactionProvider: 'static + Clone {
    type Tx<'a>: Transactional;

    fn execute<R, F>(&self, func: F) -> Result<R, DatabaseError>
    where
        F: for<'a> FnOnce(Self::Tx<'a>) -> R;
}
```

## Available Test Cases

The test suite includes comprehensive tests for:

- **Basic CRUD Operations**: Create, Read, Update, Delete entities
- **Entity Relationships**: Testing edges between entities (User-Post-Tag relationships)
- **Unique Constraints**: Email uniqueness validation (partially implemented)
- **Concurrent Updates**: Race condition testing with optimistic locking
- **Error Handling**: Proper error responses for invalid operations
- **Multiple Entity Operations**: Bulk operations and isolation

### Admin Tests

For backends that support admin operations (implementing `AdminEnt`), additional tests are available:

- **Audit Operations**: Verify edge consistency
- **Fix Operations**: Repair edge mismatches
- **List Operations**: Paginated entity listing

## Test Entities

The suite provides several test entities:

- `TestEntity`: Basic entity with name and value fields
- `User`: User entity for relationship testing
- `Post`: Post entity with author and tag relationships
- `Tag`: Tag entity for categorization
- `UserWithUniqueEmail`: User with unique email constraints

## Using the Test Suite

### Basic Tests

To run all basic tests with a `TransactionProvider`:

```rust
use ents_test_suite::run_all_tests;

let db = SqliteDb::new(pool);
run_all_tests(db)?;
```

### Admin Tests

For backends that support admin operations:

```rust
use ents_test_suite::run_audit_tests;

let db = SqliteDb::new(pool);
run_audit_tests(db)?;
```

### Example: SQLite

```rust
use ents_sqlite::SqliteDb;
use ents_test_suite::{run_all_tests, run_audit_tests};

let pool = Pool::new(SqliteConnectionManager::memory())?;
// ... setup tables ...
let db = SqliteDb::new(pool);

run_all_tests(db.clone())?;
run_audit_tests(db)?;
```

### Example: Heed (LMDB)

```rust
use ents_heed::HeedEnv;
use ents_test_suite::run_all_tests;

let env = HeedEnv::open(db_path, None)?;
run_all_tests(env)?;
```

## Individual Test Functions

Individual test functions are also available:

- `test_basic_create`
- `test_basic_read`
- `test_basic_update`
- `test_basic_delete`
- `test_relationships`
- `test_unique_constraints`
- `test_concurrent_updates`
- `test_error_handling`
- `test_multiple_entities`

Admin test functions (require `AdminEnt`):

- `test_list_entities`
- `test_audit_success`
- `test_audit_entity_not_found`
- `test_audit_unexpected_entity_type`
- `test_audit_edge_mismatch_missing_edge`
- `test_audit_edge_mismatch_extra_edge`
- `test_audit_edge_mismatch_wrong_content`
- `test_audit_null_edge_provider`
- `test_fix_ent_edges`

## Current Status

- Basic CRUD operations fully tested and working
- Entity relationships (edges) implemented and tested
- Unique constraints partially implemented (framework exists but enforcement may vary by backend)
- Concurrent updates and race condition testing
- Error handling and edge cases covered
- Multiple entity operations tested
- Admin operations (audit, fix, list) tested for supporting backends

## Future Enhancements

- Full unique constraint enforcement across all backends
- Performance benchmarking tests
- Advanced concurrent transaction testing (distributed scenarios)
- Schema migration testing
- Advanced query testing (beyond basic edge traversal)
