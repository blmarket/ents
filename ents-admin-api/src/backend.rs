use ents::{
    DatabaseError, Edge, EdgeQuery, EdgeQueryResult, EdgeValue, Ent, Id,
    QueryEdge, ReadEnt, TransactionProvider, Transactional,
};
use ents_admin::AdminEnt;

use crate::error::ApiError;
use crate::type_registry::TypeRegistry;

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
    ) -> Result<EdgeQueryResult, ApiError>;
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

    fn list_entities(
        &self,
        entity_type: &str,
        cursor: Option<Id>,
        limit: usize,
    ) -> Result<Vec<Box<dyn Ent>>, ApiError>;

    // Type discovery
    fn known_types(&self) -> Vec<String>;
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

/// A wrapper that provides [`AdminBackend`] implementation for any
/// [`TransactionProvider`] whose transaction type implements [`AdminEnt`].
///
/// This allows using any compatible storage backend with the admin API
/// without needing to implement `AdminBackend` manually.
///
/// # Type Parameters
/// * `P` - A type implementing [`TransactionProvider`]
///
/// # Requirements
/// The provider's transaction type `P::Tx<'_>` must implement [`AdminEnt`].
pub struct AdminBackendProvider<P>
where
    P: TransactionProvider,
    for<'a> P::Tx<'a>: AdminEnt,
{
    provider: P,
    type_registry: TypeRegistry<P::Tx<'static>>,
}

// Manual Clone implementation - P must be Clone, but P::Tx doesn't need to be
impl<P> Clone for AdminBackendProvider<P>
where
    P: TransactionProvider + Clone,
    for<'a> P::Tx<'a>: AdminEnt,
{
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            type_registry: self.type_registry.clone(),
        }
    }
}

impl<P> AdminBackendProvider<P>
where
    P: TransactionProvider,
    for<'a> P::Tx<'a>: AdminEnt,
{
    /// Create a new `AdminBackendProvider` with the given provider and type registry.
    pub fn new(
        provider: P,
        type_registry: TypeRegistry<P::Tx<'static>>,
    ) -> Self {
        Self {
            provider,
            type_registry,
        }
    }

    /// Set the type registry.
    pub fn with_registry(
        mut self,
        registry: TypeRegistry<P::Tx<'static>>,
    ) -> Self {
        self.type_registry = registry;
        self
    }

    /// Get a reference to the underlying provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Get a reference to the type registry.
    pub fn type_registry(&self) -> &TypeRegistry<P::Tx<'static>> {
        &self.type_registry
    }
}

impl<P> AdminBackend for AdminBackendProvider<P>
where
    P: TransactionProvider + Clone + Send + Sync + 'static,
    for<'a> P::Tx<'a>: AdminEnt,
{
    fn get_entity(&self, id: Id) -> Result<Option<Box<dyn Ent>>, ApiError> {
        self.provider
            .execute(|tx| tx.get(id))
            .map_err(ApiError::from)?
            .map_err(ApiError::from)
    }

    fn find_edges(
        &self,
        source: Id,
        query: EdgeQuery,
    ) -> Result<EdgeQueryResult, ApiError> {
        self.provider
            .execute(|tx| tx.find_edges(source, query))
            .map_err(ApiError::from)?
            .map_err(ApiError::from)
    }

    fn find_edges_by_dest(&self, dest: Id) -> Result<Vec<Edge>, ApiError> {
        self.provider
            .execute(|tx| tx.find_edges_by_dest(dest))
            .map_err(ApiError::from)?
            .map_err(ApiError::from)
    }

    fn create_entity(&self, entity: Box<dyn Ent>) -> Result<Id, ApiError> {
        self.provider
            .execute(|tx| {
                let id = tx.create_dyn(entity)?;
                tx.commit()?;
                Ok::<_, DatabaseError>(id)
            })
            .map_err(ApiError::from)?
            .map_err(ApiError::from)
    }

    fn update_entity(
        &self,
        id: Id,
        mut entity: Box<dyn Ent>,
    ) -> Result<Box<dyn Ent>, ApiError> {
        // Verify entity exists
        let existing = self.get_entity(id)?;
        if existing.is_none() {
            return Err(ApiError::NotFound(id));
        }

        // Set ID and mark updated
        entity.set_id(id);
        entity.mark_updated().map_err(|e| {
            ApiError::Internal(format!(
                "Failed to mark entity as updated: {}",
                e
            ))
        })?;

        self.provider
            .execute(|tx| {
                tx.update_dyn(entity)?;
                tx.commit()?;
                Ok::<_, DatabaseError>(())
            })
            .map_err(ApiError::from)?
            .map_err(ApiError::from)?;

        // Return updated entity
        self.get_entity(id)?.ok_or(ApiError::NotFound(id))
    }

    fn delete_entity(&self, id: Id) -> Result<(), ApiError> {
        // Verify entity exists
        let existing = self.get_entity(id)?;
        if existing.is_none() {
            return Err(ApiError::NotFound(id));
        }

        self.provider
            .execute(|tx| {
                // Remove incoming edges
                tx.remove_edges_by_dest(id)?;
                // Delete the entity
                tx.delete(id)?;
                tx.commit()?;
                Ok::<_, DatabaseError>(())
            })
            .map_err(ApiError::from)?
            .map_err(ApiError::from)
    }

    fn audit_entity_edges(
        &self,
        id: Id,
        type_name: &str,
    ) -> Result<AuditResult, ApiError> {
        let (audit_fn, _) =
            self.type_registry.get(type_name).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "Unknown entity type: {}. Available types: {}",
                    type_name,
                    self.type_registry.available_types().join(", ")
                ))
            })?;

        self.provider
            .execute(|tx| {
                // SAFETY: The audit_fn closure doesn't store references beyond
                // the duration of this call. The transaction is dropped after.
                let tx_static: &P::Tx<'static> =
                    unsafe { std::mem::transmute(&tx) };
                audit_fn(tx_static, id)
            })
            .map_err(ApiError::from)?
    }

    fn fix_entity_edges(
        &self,
        id: Id,
        type_name: &str,
    ) -> Result<(), ApiError> {
        let (_, fix_fn) =
            self.type_registry.get(type_name).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "Unknown entity type: {}. Available types: {}",
                    type_name,
                    self.type_registry.available_types().join(", ")
                ))
            })?;

        self.provider
            .execute(|tx| {
                // SAFETY: The fix_fn closure consumes the transaction and commits it.
                // The closure doesn't store references beyond this call.
                let tx_static: P::Tx<'static> =
                    unsafe { std::mem::transmute(tx) };
                fix_fn(tx_static, id)
            })
            .map_err(ApiError::from)?
    }

    fn list_entities(
        &self,
        entity_type: &str,
        cursor: Option<Id>,
        limit: usize,
    ) -> Result<Vec<Box<dyn Ent>>, ApiError> {
        self.provider
            .execute(|tx| tx.list_entities(entity_type, cursor, limit))
            .map_err(ApiError::from)?
            .map_err(ApiError::from)
    }

    fn known_types(&self) -> Vec<String> {
        self.type_registry
            .available_types()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}
