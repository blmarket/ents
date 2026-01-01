//! Edge provider traits and implementations for managing edges between entities.
//!
//! This module provides a type-safe way to define and validate edges between entities
//! before they are inserted into the database.

use std::borrow::BorrowMut;

use crate::query_edge::QueryEdge;
use crate::{DatabaseError, Ent, Id};

/// Represents a validated edge ready to be inserted into the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeValue {
    /// The source entity ID
    pub source: Id,
    /// The edge type (as bytes, can be binary)
    pub sort_key: Vec<u8>,
    /// The destination entity ID
    pub dest: Id,
}

impl EdgeValue {
    /// Create a new EdgeValue
    pub fn new(source: Id, sort_key: Vec<u8>, dest: Id) -> Self {
        Self {
            source,
            sort_key,
            dest,
        }
    }
}

/// Errors that can occur when creating an edge draft
#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    #[error("Source entity not found: {0}")]
    SourceNotFound(Id),

    #[error("Destination entity not found: {0}")]
    DestNotFound(Id),

    #[error("Invalid edge type: {0}")]
    InvalidEdgeType(String),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

pub trait EdgeDraft: PartialEq {
    fn check<T: Transactional>(self, txn: &T) -> Result<Vec<EdgeValue>, DraftError>;
}

pub trait EdgeProvider<E: Ent + ?Sized> {
    type Draft: EdgeDraft;

    fn draft(ent: &E) -> Self::Draft;
}

pub trait EntWithEdges: Ent {
    type EdgeProvider: EdgeProvider<Self>;

    fn setup_edges<T: Transactional>(&self, txn: &T) -> Result<(), DraftError> {
        let draft = Self::EdgeProvider::draft(self);
        for edge in draft.check(txn)? {
            txn.create_edge(edge)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullEdgeDraft;

impl EdgeDraft for NullEdgeDraft {
    fn check<T: Transactional + QueryEdge>(
        self,
        _txn: &T,
    ) -> Result<Vec<EdgeValue>, DraftError> {
        Ok(Vec::new())
    }
}

/// A no-op edge provider for entities that don't have edges.
pub struct NullEdgeProvider;

impl<E: Ent> EdgeProvider<E> for NullEdgeProvider {
    type Draft = NullEdgeDraft;

    fn draft(_ent: &E) -> Self::Draft {
        NullEdgeDraft
    }
}

/// A trait for abstracting database transactions and operations.
///
/// This trait provides a unified interface for performing CRUD (Create, Read, Update, Delete)
/// operations on entities and edges, as well as querying relationships.
/// It abstracts over the underlying storage and transaction management, allowing
/// code to work with entities without being tightly coupled to a specific database backend.
///
/// # Key Features
///
/// - **Entity Management**: `insert`, `get`, `remove`, `update` entities.
/// - **Edge Management**: `add_edge`, `remove_edge`.
/// - **Querying**: Find edges (`find_edge`, `find_edges_in`), find entities by type (`find_by_type`).
/// - **Concurrency Control**: `update` supports optimistic concurrency control via CAS (Compare-And-Set).
pub trait Transactional: QueryEdge {
    fn get(&self, id: Id) -> Result<Option<Box<dyn Ent>>, DatabaseError>;

    fn create<E: EntWithEdges>(&self, ent: E) -> Result<Id, DatabaseError>;

    fn delete<E: EntWithEdges>(&self, id: Id) -> Result<(), DatabaseError>;

    fn create_edge(&self, edge: EdgeValue) -> Result<(), DatabaseError>;

    fn update<T, F, B>(&self, ent: B, mutator: F) -> Result<bool, DatabaseError>
    where
        T: EntWithEdges,
        F: FnOnce(&mut T),
        B: BorrowMut<T>;

    fn commit(self) -> Result<(), DatabaseError>;
}

impl<T1, T2> EdgeDraft for (T1, T2)
where
    T1: EdgeDraft,
    T2: EdgeDraft,
{
    fn check<Trans: Transactional>(self, txn: &Trans) -> Result<Vec<EdgeValue>, DraftError> {
        let (t1, t2) = self;
        let mut edges = t1.check(txn)?;
        edges.extend(t2.check(txn)?);
        Ok(edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_value_creation() {
        let edge = EdgeValue::new(1, b"connects_to".to_vec(), 2);
        assert_eq!(edge.source, 1);
        assert_eq!(edge.sort_key, b"connects_to");
        assert_eq!(edge.dest, 2);
    }
}
