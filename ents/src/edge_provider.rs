//! Edge provider traits and implementations for managing edges between entities.
//!
//! This module provides a type-safe way to define and validate edges between entities
//! before they are inserted into the database.

use crate::{DatabaseError, Ent, Id, ReadEnt};

/// Represents an incoming edge without destination.
///
/// The destination is always the owning entity and is attached by
/// [`IncomingEdgeProvider`] during edge materialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingEdgeValue {
    /// The source entity ID
    pub source: Id,
    /// The edge type (as bytes, can be binary)
    pub sort_key: Vec<u8>,
}

impl IncomingEdgeValue {
    /// Create a new IncomingEdgeValue
    pub fn new(source: Id, sort_key: Vec<u8>) -> Self {
        Self { source, sort_key }
    }

    /// Convert into a full edge value with a fixed destination.
    pub fn with_dest(self, dest: Id) -> EdgeValue {
        EdgeValue::new(self.source, self.sort_key, dest)
    }
}

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

/// Draft of incoming edges before validation.
pub trait EdgeDraft: PartialEq {
    fn check<T: ReadEnt>(
        self,
        txn: &T,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError>;
}

/// incoming edges whose destination is the entity.
pub trait IncomingEdgeProvider<E: Ent + ?Sized> {
    /// Generated edge draft type
    type Draft: EdgeDraft;

    /// Draft edges from the entity value
    fn draft(ent: &E) -> Self::Draft;
}

/// Validate and materialize incoming edges for an entity.
///
/// This guarantees all returned edges have `dest == ent.id()`.
pub fn check_incoming_edges<E, T>(
    ent: &E,
    txn: &T,
) -> Result<Vec<EdgeValue>, DraftError>
where
    E: Ent,
    T: ReadEnt,
{
    let dest = ent.id();
    Ok(E::EdgeProvider::draft(ent)
        .check(txn)?
        .into_iter()
        .map(|edge| edge.with_dest(dest))
        .collect())
}

impl<E: Ent, T1, T2> IncomingEdgeProvider<E> for (T1, T2)
where
    T1: IncomingEdgeProvider<E>,
    T2: IncomingEdgeProvider<E>,
{
    type Draft = (T1::Draft, T2::Draft);

    fn draft(ent: &E) -> Self::Draft {
        (T1::draft(ent), T2::draft(ent))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullEdgeDraft;

impl EdgeDraft for NullEdgeDraft {
    fn check<T: ReadEnt>(
        self,
        _txn: &T,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError> {
        Ok(Vec::new())
    }
}

/// A no-op edge provider for entities that don't have edges.
pub struct NullEdgeProvider;

impl<E: Ent> IncomingEdgeProvider<E> for NullEdgeProvider {
    type Draft = NullEdgeDraft;

    fn draft(_ent: &E) -> Self::Draft {
        NullEdgeDraft
    }
}

impl<T1, T2> EdgeDraft for (T1, T2)
where
    T1: EdgeDraft,
    T2: EdgeDraft,
{
    fn check<Trans: ReadEnt>(
        self,
        txn: &Trans,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError> {
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

    #[test]
    fn test_incoming_edge_value_creation() {
        let edge = IncomingEdgeValue::new(1, b"connects_to".to_vec());
        assert_eq!(edge.source, 1);
        assert_eq!(edge.sort_key, b"connects_to");
    }
}
