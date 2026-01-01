# Ents

A minimal, type-safe entity framework for Rust that provides a simple and
extensible way to work with entities and their relationships.

This crate is inspired by the
[entity](https://lib.rs/crates/entity)
crate but rewritten from the scratch.

## Features

- Tried to be as minimal as possible. User just needs to:
  - Implement `Ent` trait for their entities
  - Define edge relationships using `EntWithEdges` trait. Unfortunately traits
    were separated due to
    [typetag](https://lib.rs/crates/typetag)
    limitations.
- Support consistency via `Transactional` API - using database transactions
  - Can implement UNIQUE constraints
  - Write all or nothing
