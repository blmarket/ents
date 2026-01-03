# ents

A minimal, type-safe entity framework for Rust that provides a simple and
extensible way to work with entities and their relationships.

## Overview

`ents` is the core crate that defines the foundational traits and types for
building entity-based applications. It provides the abstractions needed to work
with entities, edges (relationships), and database operations without being
tied to any specific storage backend.

## Features

- **Minimal API**: Simple trait-based design requiring only two main traits to implement
  - `Ent`: Core entity trait with serialization support via `typetag`
  - `EntWithEdges`: Define relationships between entities
- **Type-safe**: Leverages Rust's type system for compile-time safety
- **Storage-agnostic**: Define your entities once, use with any backend implementation
- **Edge relationships**: Built-in support for querying and managing entity relationships
- **Transactional API**: Support for ACID transactions through the `Transactional` trait

## Core Concepts

### Entities (`Ent` trait)

An entity is any type that implements the `Ent` trait. Each entity has:
- A unique identifier (`Id`)
- A timestamp for tracking updates (`last_updated`)
- Serialization support for persistence

### Edges

Edges represent relationships between entities. The framework provides:
- `EdgeProvider`: Interface for managing edges
- `EdgeQuery`: Flexible querying of relationships
- `EdgeDraft`: Transactional edge mutations

