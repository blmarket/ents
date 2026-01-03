# AGENTS.md

## Project Overview

Ents is a minimal, type-safe entity framework for Rust inspired by the [entity](https://lib.rs/crates/entity) crate. The framework provides abstractions for entities and relationships (edges) that can work with different storage backends.

This is a Cargo workspace with four crates:
- `ents`: Core framework defining traits and types (storage-agnostic)
- `ents-sqlite`: SQLite storage backend implementation
- `ents-heed`: LMDB storage backend implementation (via heed)
- `ents-test-suite`: Shared test suite for validating backend implementations

## Common Commands

### Building
```bash
# Build all workspace crates
cargo build --workspace

# Build a specific crate
cargo build -p ents
cargo build -p ents-sqlite
cargo build -p ents-heed
```

### Testing
```bash
# Run all tests in the workspace
cargo test --workspace

# Run tests for a specific crate
cargo test -p ents-sqlite
cargo test -p ents-heed

# Run a specific test by name
cargo test test_basic_create
cargo test edge_traverse

# Run tests in a specific file (from the crate directory)
cargo test --test txn
cargo test --test edge_traverse
```

### Publishing
```bash
# Check what would be published
cargo publish --dry-run -p ents

# Publish in order (dependencies first)
cargo publish -p ents
cargo publish -p ents-test-suite
cargo publish -p ents-sqlite
cargo publish -p ents-heed
```

## Architecture

### Core Concepts

**Entities**: Types implementing the `Ent` trait with:
- Unique ID (`u64`)
- Timestamp tracking (`last_updated` in microseconds)
- Serialization via `typetag::serde` for dynamic dispatch

**Edges**: Directed relationships between entities represented as `(source_id, sort_key, dest_id)` tuples where:
- `source`: Source entity ID
- `sort_key`: Binary key for edge type/ordering (e.g., `b"author"`, `b"tag"`)
- `dest`: Destination entity ID

**Edge Provider Pattern**: Entities with relationships implement `EntWithEdges` and define an `EdgeProvider` that:
1. Creates an `EdgeDraft` from entity data
2. Validates the draft (checking entity existence, enforcing constraints)
3. Returns validated `EdgeValue`s to be inserted

**Transactions**: The `Transactional` trait provides ACID operations:
- Entity CRUD: `create()`, `get()`, `delete()`, `update()`
- Edge operations: `create_edge()`
- Query operations: `find_edges()` via `QueryEdge` trait
- Optimistic concurrency control via CAS (Compare-And-Set) in `update()`

### Key Patterns

**Entity Definition**:
- Implement `Ent` trait with `#[typetag::serde]` for serialization
- Implement `EntWithEdges` trait (use `NullEdgeProvider` if no relationships)
- `mark_updated()` should set `last_updated` to current time in microseconds

**Edge Validation**:
- Edge drafts can enforce constraints (uniqueness, foreign key existence)
- Tuple composition `(Draft1, Draft2)` implements `EdgeDraft` for multiple edge types
- Source ID 0 is conventionally used for global constraints (e.g., unique emails)

**Backend Implementation**:
- Implement `Transactional` and `QueryEdge` traits
- Entities stored as serialized JSON with type information
- Edges stored with composite keys for efficient querying
- SQLite uses JSON functions for querying; LMDB uses snowflake IDs for generation

### Storage Layout

**SQLite** (`ents-sqlite`):
- `entities` table: `(id INTEGER PRIMARY KEY, type TEXT, data TEXT)`
- `edges` table: `(source INTEGER, sort_key BLOB, dest INTEGER, PRIMARY KEY(source, sort_key, dest))`
- Uses `r2d2_sqlite` for connection pooling

**LMDB** (`ents-heed`):
- `entities` database: `u64 -> String` (ID to JSON)
- `edges` database: composite key `(source, sort_key, dest)` with binary encoding
- Uses snowflake ID generation for distributed-friendly IDs
- BigEndian encoding for proper key ordering

### Test Suite Pattern

Backend implementations use `ents-test-suite` which provides:
- Test entity types: `TestEntity`, `User`, `Post`, `Tag`, `UserWithUniqueEmail`
- Reusable test functions: `test_basic_create()`, `test_relationships()`, etc.
- Traits: `TestSuiteRunner` (creates test runners) and `TestCaseRunner` (executes in transaction)

Tests are located in `<backend>/tests/` and import test functions from `ents-test-suite`.

## Important Notes

- The framework requires Rust nightly due to workspace dependencies and typetag
- `last_updated` is in **microseconds** (not milliseconds or seconds)
- Edge queries return up to 100 edges max
- Entity IDs must be set by the backend's `create()` method (set to 0 before insert)
- The `Transactional::update()` method performs CAS updates for concurrency control
- Edge sort keys are binary (Vec<u8>) to support custom ordering schemes
