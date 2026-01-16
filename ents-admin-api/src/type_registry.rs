use std::collections::HashMap;
use std::sync::Arc;

use ents::{EdgeDraft, EdgeValue, Ent, EntExt, Id, IncomingEdgeProvider};
use ents_admin::AdminEnt;

use crate::backend::{AuditResult, EdgeInfo};
use crate::error::ApiError;

/// Type alias for audit functions that work with any transaction type.
pub type AuditFn<T> =
    Arc<dyn Fn(&T, Id) -> Result<AuditResult, ApiError> + Send + Sync>;

/// Type alias for fix functions that work with any transaction type.
pub type FixFn<T> = Arc<dyn Fn(T, Id) -> Result<(), ApiError> + Send + Sync>;

/// Registry of entity types for audit/fix operations.
///
/// This registry stores closures that operate on a transaction type `T`
/// which must implement `AdminEdgeByDest`.
///
/// Use [`TypeRegistryBuilder`] to construct a registry.
pub struct TypeRegistry<T> {
    entries: Arc<HashMap<String, (AuditFn<T>, FixFn<T>)>>,
}

// Manual Clone implementation since T doesn't need to be Clone
// (we only store Arc-wrapped closures, not T itself)
impl<T> Clone for TypeRegistry<T> {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
        }
    }
}

impl<T> Default for TypeRegistry<T> {
    fn default() -> Self {
        Self {
            entries: Arc::new(HashMap::new()),
        }
    }
}

/// Builder for constructing a [`TypeRegistry`].
pub struct TypeRegistryBuilder<T> {
    entries: HashMap<String, (AuditFn<T>, FixFn<T>)>,
}

impl<T> Default for TypeRegistryBuilder<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T> TypeRegistryBuilder<T>
where
    T: AdminEnt,
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

        let audit_fn: AuditFn<T> = Arc::new(move |txn: &T, id: Id| {
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
                <E::EdgeProvider as IncomingEdgeProvider<E>>::draft(ent);
            let mut expected_edges = draft.check(txn).map_err(|e| {
                ApiError::Internal(format!("Failed to draft edges: {}", e))
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
        });

        let fix_fn: FixFn<T> = Arc::new(move |txn: T, id: Id| {
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
                e => ApiError::Internal(format!("Failed to fix edges: {}", e)),
            })
        });

        self.entries.insert(type_name, (audit_fn, fix_fn));
        self
    }

    /// Build the [`TypeRegistry`].
    pub fn build(self) -> TypeRegistry<T> {
        TypeRegistry {
            entries: Arc::new(self.entries),
        }
    }
}

impl<T> TypeRegistry<T> {
    pub fn get(&self, type_name: &str) -> Option<&(AuditFn<T>, FixFn<T>)> {
        self.entries.get(type_name)
    }

    pub fn available_types(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}
