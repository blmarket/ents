use ents::{Edge, EdgeQuery, EdgeValue, Ent, Id};

use crate::error::ApiError;

/// Backend trait for admin API operations.
///
/// This trait abstracts over the underlying storage and transaction management,
/// allowing the admin API to work with any backend that implements these operations.
///
/// Implementations should handle transaction lifecycle internally:
/// - Read operations can use any transaction/connection
/// - Write operations should begin a transaction, perform the operation, and commit
pub trait AdminBackend: Clone + Send + Sync + 'static {
    // Read operations
    fn get_entity(&self, id: Id) -> Result<Option<Box<dyn Ent>>, ApiError>;
    fn find_edges(
        &self,
        source: Id,
        query: EdgeQuery,
    ) -> Result<Vec<Edge>, ApiError>;
    fn find_edges_by_dest(&self, dest: Id) -> Result<Vec<Edge>, ApiError>;

    // Write operations (should handle transaction lifecycle internally)
    fn create_entity(&self, entity: Box<dyn Ent>) -> Result<Id, ApiError>;
    fn update_entity(
        &self,
        id: Id,
        entity: Box<dyn Ent>,
    ) -> Result<Box<dyn Ent>, ApiError>;
    fn delete_entity(&self, id: Id) -> Result<(), ApiError>;

    // Edge audit operations
    fn audit_entity_edges(
        &self,
        id: Id,
        type_name: &str,
    ) -> Result<AuditResult, ApiError>;
    fn fix_entity_edges(&self, id: Id, type_name: &str)
        -> Result<(), ApiError>;
}

/// Result of an edge audit operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditResult {
    pub valid: bool,
    pub existing_edges: Vec<EdgeInfo>,
    pub expected_edges: Vec<EdgeInfo>,
    pub missing: Vec<EdgeInfo>,
    pub extra: Vec<EdgeInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EdgeInfo {
    pub source: Id,
    pub sort_key: String,
    pub sort_key_bytes: Vec<u8>,
    pub dest: Id,
}

impl From<EdgeValue> for EdgeInfo {
    fn from(e: EdgeValue) -> Self {
        Self {
            source: e.source,
            sort_key: String::from_utf8_lossy(&e.sort_key).to_string(),
            sort_key_bytes: e.sort_key,
            dest: e.dest,
        }
    }
}

impl From<Edge> for EdgeInfo {
    fn from(e: Edge) -> Self {
        Self {
            source: e.source,
            sort_key: String::from_utf8_lossy(&e.sort_key).to_string(),
            sort_key_bytes: e.sort_key,
            dest: e.dest,
        }
    }
}
