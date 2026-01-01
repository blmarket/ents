# Ents Test Suite Agents

The `ents-test-suite` is a comprehensive test suite designed to validate implementations of the `ents` entity framework. This document describes the agent system and how to implement or use different test runners.

## Overview

The test suite uses a generic architecture where "agents" are implementations that provide concrete database/storage backends for the `ents` framework. Each agent implements the `TestSuiteRunner` trait, which allows the test suite to run the same tests against different storage engines.

## Agent Interface

### TestSuiteRunner Trait

```rust
pub trait TestSuiteRunner: Clone {
    type CaseRunner: TestCaseRunner;

    fn create(&self) -> anyhow::Result<Self::CaseRunner>;
}
```

### TestCaseRunner Trait

```rust
pub trait TestCaseRunner {
    type Tx: Transactional;

    fn execute<F, R>(&mut self, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(Self::Tx) -> anyhow::Result<R>;
}
```

## Available Test Cases

The test suite includes comprehensive tests for:

- **Basic CRUD Operations**: Create, Read, Update, Delete entities
- **Entity Relationships**: Testing edges between entities (User-Post-Tag relationships)
- **Unique Constraints**: Email uniqueness validation (partially implemented)
- **Error Handling**: Proper error responses for invalid operations
- **Multiple Entity Operations**: Bulk operations and isolation

## Test Entities

The suite provides several test entities:

- `TestEntity`: Basic entity with name and value fields
- `User`: User entity for relationship testing
- `Post`: Post entity with author and tag relationships
- `Tag`: Tag entity for categorization
- `UserWithUniqueEmail`: User with unique email constraints

## Implementing a New Agent

To implement a new agent for a different storage backend:

1. **Implement Transactional**: Your storage backend must implement the `Transactional` trait from the `ents` crate.

2. **Create a TestCaseRunner**: Implement `TestCaseRunner` where `Tx` is your transactional type.

3. **Create a TestSuiteRunner**: Implement `TestSuiteRunner` that creates instances of your `TestCaseRunner`.

4. **Run the tests**: Use `run_all_tests(your_runner)` to execute the full test suite.

### Example Agent Structure

```rust
struct MyDatabaseAgent {
    connection_string: String,
}

impl TestSuiteRunner for MyDatabaseAgent {
    type CaseRunner = MyDatabaseTestRunner;

    fn create(&self) -> anyhow::Result<Self::CaseRunner> {
        // Initialize your database connection
        let connection = MyDatabase::connect(&self.connection_string)?;
        Ok(MyDatabaseTestRunner { connection })
    }
}

struct MyDatabaseTestRunner {
    connection: MyDatabaseConnection,
}

impl TestCaseRunner for MyDatabaseTestRunner {
    type Tx = MyDatabaseTransaction;

    fn execute<F, R>(&mut self, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(Self::Tx) -> anyhow::Result<R>,
    {
        let transaction = self.connection.begin_transaction()?;
        let result = f(transaction);
        // Handle commit/rollback based on result
        result
    }
}
```

## Running Tests

To run all tests with an agent:

```rust
use ents_test_suite::run_all_tests;

let agent = MyDatabaseAgent::new("connection_string");
run_all_tests(agent)?;
```

Individual test functions are also available:

- `test_basic_create`
- `test_basic_read`
- `test_basic_update`
- `test_basic_delete`
- `test_relationships`
- `test_unique_constraints`
- `test_error_handling`
- `test_multiple_entities`

## Current Status

- ✅ Basic CRUD operations fully tested and working
- ✅ Entity relationships (edges) implemented and tested
- ⚠️ Unique constraints partially implemented (framework exists but enforcement may vary by backend)
- ✅ Error handling and edge cases covered
- ✅ Multiple entity operations tested

## Future Enhancements

- Full unique constraint enforcement across all backends
- Performance benchmarking tests
- Concurrent transaction testing
- Schema migration testing
- Advanced query testing (beyond basic edge traversal)