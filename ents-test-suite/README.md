# ents-test-suite

Shared test suite for validating [ents](../ents) database backend
implementations.

## Overview

`ents-test-suite` provides a comprehensive, reusable set of test cases and test entities that ensure any storage backend implementation correctly implements the `ents` framework contracts. This enables consistent behavior across different database backends.

## Usage

This crate is typically used as a dev-dependency for backend implementations:

```toml
[dev-dependencies]
ents-test-suite = { path = "../ents-test-suite" }
```

### Running Tests

Implement the `TestSuiteRunner` and `TestCaseRunner` traits for your backend, then use the provided test functions:

```rust
use ents_test_suite::{test_basic_create, TestSuiteRunner};

#[test]
fn test_create() {
    let runner = MyBackendRunner::new();
    test_basic_create(&runner).unwrap();
}
```

## Test Traits

- `TestSuiteRunner`: Creates test case runners for your backend
- `TestCaseRunner`: Executes individual test cases with transaction support

## Examples

See the test implementations in:
- [ents-sqlite](../ents-sqlite/tests)
- [ents-heed](../ents-heed/tests)

