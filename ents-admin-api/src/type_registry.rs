use std::collections::HashMap;
use std::sync::Arc;

use ents::{
    EdgeDraft, EdgeValue, Ent, EntExt, Id, IncomingEdgeProvider, ReadEnt,
    TransactionProvider,
};
use ents_admin::AdminEnt;

use crate::backend::{AuditResult, EdgeInfo};
use crate::error::ApiError;

/// Type alias for audit functions that work with any provider type.
pub type AuditFn<P> =
    Arc<dyn Fn(&P, Id) -> Result<AuditResult, ApiError> + Send + Sync>;

/// Type alias for fix functions that work with any provider type.
pub type FixFn<P> = Arc<dyn Fn(&P, Id) -> Result<(), ApiError> + Send + Sync>;

/// Registry of entity types for audit/fix operations.
///
/// Use [`TypeRegistryBuilder`] to construct a registry.
#[derive(Clone, Default)]
pub struct TypeRegistry<P> {
    entries: Arc<HashMap<String, (AuditFn<P>, FixFn<P>)>>,
}

/// Builder for constructing a [`TypeRegistry`].
pub struct TypeRegistryBuilder<P> {
    entries: HashMap<String, (AuditFn<P>, FixFn<P>)>,
}

impl<P> Default for TypeRegistryBuilder<P> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<P> TypeRegistryBuilder<P>
where
    P: TransactionProvider + 'static,
    for<'a> P::Tx<'a>: AdminEnt,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an entity type for audit/fix operations.
    ///
    /// This allows the admin API to perform edge auditing and fixing
    /// for the registered entity type.
    pub fn register<E: Ent>(mut self) -> Self {
        let type_name = std::any::type_name::<E>()
            .rsplit("::")
            .next()
            .unwrap_or("Unknown")
            .to_string();

        let audit_fn: AuditFn<P> = Arc::new(move |provider: &P, id: Id| {
            provider
                .execute(|txn| {
                    let ent_box = txn.get(id)?.ok_or(ApiError::NotFound(id))?;

                    let ent = ent_box.as_ent::<E>().ok_or_else(|| {
                        ApiError::BadRequest(format!(
                            "Entity {} is not of type {}",
                            id,
                            std::any::type_name::<E>()
                        ))
                    })?;

                    // Find existing incoming edges
                    let mut existing_edges: Vec<EdgeValue> = txn
                        .find_edges_by_dest(id)?
                        .into_iter()
                        .map(|e| EdgeValue::new(e.source, e.sort_key, e.dest))
                        .collect();
                    existing_edges.sort_by(|a, b| {
                        (&a.source, &a.sort_key).cmp(&(&b.source, &b.sort_key))
                    });

                    // Draft expected edges
                    let draft =
                        <E::EdgeProvider as IncomingEdgeProvider<E>>::draft(
                            ent,
                        );
                    let mut expected_edges =
                        draft.check(&txn).map_err(|e| {
                            ApiError::Internal(format!(
                                "Failed to draft edges: {}",
                                e
                            ))
                        })?;
                    expected_edges.sort_by(|a, b| {
                        (&a.source, &a.sort_key).cmp(&(&b.source, &b.sort_key))
                    });

                    let valid = existing_edges == expected_edges;

                    // Calculate missing and extra edges
                    let missing: Vec<EdgeInfo> = expected_edges
                        .iter()
                        .filter(|e| !existing_edges.contains(e))
                        .cloned()
                        .map(EdgeInfo::from)
                        .collect();

                    let extra: Vec<EdgeInfo> = existing_edges
                        .iter()
                        .filter(|e| !expected_edges.contains(e))
                        .cloned()
                        .map(EdgeInfo::from)
                        .collect();

                    Ok(AuditResult {
                        valid,
                        existing_edges: existing_edges
                            .into_iter()
                            .map(EdgeInfo::from)
                            .collect(),
                        expected_edges: expected_edges
                            .into_iter()
                            .map(EdgeInfo::from)
                            .collect(),
                        missing,
                        extra,
                    })
                })
                .map_err(ApiError::from)?
        });

        let fix_fn: FixFn<P> = Arc::new(move |provider: &P, id: Id| {
            provider
                .execute(|txn| {
                    txn.fix_ent_edges::<E>(id).map_err(|e| match e {
                        ents_admin::AuditError::EntityNotFound(id) => {
                            ApiError::NotFound(id)
                        }
                        ents_admin::AuditError::UnexpectedEntityType(id, t) => {
                            ApiError::BadRequest(format!(
                                "Entity {} is not of type {}",
                                id, t
                            ))
                        }
                        e => ApiError::Internal(format!(
                            "Failed to fix edges: {}",
                            e
                        )),
                    })
                })
                .map_err(ApiError::from)?
        });

        self.entries.insert(type_name, (audit_fn, fix_fn));
        self
    }

    /// Build the [`TypeRegistry`].
    pub fn build(self) -> TypeRegistry<P> {
        TypeRegistry {
            entries: Arc::new(self.entries),
        }
    }
}

impl<P> TypeRegistry<P> {
    pub fn get(&self, type_name: &str) -> Option<&(AuditFn<P>, FixFn<P>)> {
        self.entries.get(type_name)
    }

    pub fn available_types(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}
