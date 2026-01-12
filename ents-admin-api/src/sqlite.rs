use std::collections::HashMap;
use std::sync::Arc;

use ents::{
    Edge, EdgeDraft, EdgeQuery, EdgeValue, Ent, EntExt, Id,
    IncomingEdgeProvider, QueryEdge, ReadEnt,
};
use ents_admin::AdminEdgeByDest;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::rusqlite::params;
use r2d2_sqlite::SqliteConnectionManager;

use crate::backend::{AdminBackend, AuditResult, EdgeInfo};
use crate::error::ApiError;

/// SQLite connection pool wrapper for admin API operations.
#[derive(Clone)]
pub struct SqlitePool {
    pool: Arc<Pool<SqliteConnectionManager>>,
    type_registry: TypeRegistry,
}

/// Registry of entity types for audit/fix operations.
///
/// Since Rust's type system requires concrete types at compile time for
/// `audit_ent_edges` and `fix_ent_edges`, this registry stores closures
/// that operate on the SQLite transaction type directly.
///
/// Use [`TypeRegistryBuilder`] to construct a registry.
#[derive(Clone, Default)]
pub struct TypeRegistry {
    entries: Arc<HashMap<String, (AuditFn, FixFn)>>,
}

type AuditFn = Arc<
    dyn Fn(&ents_sqlite::Txn, Id) -> Result<AuditResult, ApiError>
        + Send
        + Sync,
>;
type FixFn =
    Arc<dyn Fn(ents_sqlite::Txn, Id) -> Result<(), ApiError> + Send + Sync>;

/// Builder for constructing a [`TypeRegistry`].
#[derive(Default)]
pub struct TypeRegistryBuilder {
    entries: HashMap<String, (AuditFn, FixFn)>,
}

impl TypeRegistryBuilder {
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

        let audit_fn: AuditFn =
            Arc::new(move |txn: &ents_sqlite::Txn, id: Id| {
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

        let fix_fn: FixFn = Arc::new(move |txn: ents_sqlite::Txn, id: Id| {
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
    pub fn build(self) -> TypeRegistry {
        TypeRegistry {
            entries: Arc::new(self.entries),
        }
    }
}

impl TypeRegistry {
    fn get(&self, type_name: &str) -> Option<&(AuditFn, FixFn)> {
        self.entries.get(type_name)
    }

    fn available_types(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}

impl SqlitePool {
    /// Create a new SQLite pool with the given database path.
    pub fn open(path: &str) -> Result<Self, r2d2::Error> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager)?;
        Ok(Self {
            pool: Arc::new(pool),
            type_registry: TypeRegistry::default(),
        })
    }

    /// Create a new in-memory SQLite pool.
    pub fn open_in_memory() -> Result<Self, r2d2::Error> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::new(manager)?;
        Ok(Self {
            pool: Arc::new(pool),
            type_registry: TypeRegistry::default(),
        })
    }

    /// Create from an existing r2d2 pool.
    pub fn from_pool(pool: Pool<SqliteConnectionManager>) -> Self {
        Self {
            pool: Arc::new(pool),
            type_registry: TypeRegistry::default(),
        }
    }

    /// Initialize database schema.
    pub fn init_schema(&self) -> Result<(), r2d2_sqlite::rusqlite::Error> {
        let conn = self.pool.get().map_err(|e| {
            r2d2_sqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                data TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS edges (
                source INTEGER NOT NULL,
                type BLOB NOT NULL,
                dest INTEGER NOT NULL,
                PRIMARY KEY (source, type, dest)
            );
            CREATE INDEX IF NOT EXISTS edges_dest ON edges(dest);
            "#,
        )
    }

    /// Set the type registry.
    pub fn with_registry(mut self, registry: TypeRegistry) -> Self {
        self.type_registry = registry;
        self
    }

    /// Register an entity type for audit/fix operations.
    ///
    /// This is a convenience method that allows registering types one at a time.
    /// For registering multiple types, consider using [`TypeRegistryBuilder`].
    pub fn register_type<E: Ent>(&mut self) {
        let type_name = std::any::type_name::<E>()
            .rsplit("::")
            .next()
            .unwrap_or("Unknown")
            .to_string();

        let audit_fn: AuditFn =
            Arc::new(move |txn: &ents_sqlite::Txn, id: Id| {
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

        let fix_fn: FixFn = Arc::new(move |txn: ents_sqlite::Txn, id: Id| {
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

        // Clone the existing entries and add new one
        let mut entries = (*self.type_registry.entries).clone();
        entries.insert(type_name, (audit_fn, fix_fn));
        self.type_registry = TypeRegistry {
            entries: Arc::new(entries),
        };
    }

    fn get_conn(
        &self,
    ) -> Result<PooledConnection<SqliteConnectionManager>, ApiError> {
        self.pool.get().map_err(|e| {
            ApiError::Internal(format!("Failed to get connection: {}", e))
        })
    }

    fn with_txn<F, R>(&self, f: F) -> Result<R, ApiError>
    where
        F: FnOnce(&ents_sqlite::Txn) -> Result<R, ApiError>,
    {
        let mut conn = self.get_conn()?;
        let txn = ents_sqlite::Txn::new(conn.transaction().map_err(|e| {
            ApiError::Internal(format!("Failed to start transaction: {}", e))
        })?);
        f(&txn)
    }
}

impl AdminBackend for SqlitePool {
    fn get_entity(&self, id: Id) -> Result<Option<Box<dyn Ent>>, ApiError> {
        self.with_txn(|txn| txn.get(id).map_err(ApiError::from))
    }

    fn find_edges(
        &self,
        source: Id,
        query: EdgeQuery,
    ) -> Result<Vec<Edge>, ApiError> {
        self.with_txn(|txn| {
            txn.find_edges(source, query).map_err(ApiError::from)
        })
    }

    fn find_edges_by_dest(&self, dest: Id) -> Result<Vec<Edge>, ApiError> {
        self.with_txn(|txn| {
            txn.find_edges_by_dest(dest).map_err(ApiError::from)
        })
    }

    fn create_entity(&self, entity: Box<dyn Ent>) -> Result<Id, ApiError> {
        let mut conn = self.get_conn()?;
        let txn = conn.transaction().map_err(|e| {
            ApiError::Internal(format!("Failed to start transaction: {}", e))
        })?;

        // Serialize and insert entity
        let entity_type = entity.typetag_name().to_string();
        let data_json = serde_json::to_string(&entity)?;

        txn.execute(
            "INSERT INTO entities (type, data) VALUES (?1, ?2)",
            params![entity_type, data_json],
        )
        .map_err(|e| {
            ApiError::Internal(format!("Failed to insert entity: {}", e))
        })?;

        let id = txn.last_insert_rowid() as Id;

        txn.commit().map_err(|e| {
            ApiError::Internal(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(id)
    }

    fn update_entity(
        &self,
        id: Id,
        mut entity: Box<dyn Ent>,
    ) -> Result<Box<dyn Ent>, ApiError> {
        let mut conn = self.get_conn()?;
        let txn = conn.transaction().map_err(|e| {
            ApiError::Internal(format!("Failed to start transaction: {}", e))
        })?;

        // Verify entity exists
        let _: i64 = txn
            .query_row(
                "SELECT id FROM entities WHERE id = ?1",
                params![id as i64],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                r2d2_sqlite::rusqlite::Error::QueryReturnedNoRows => {
                    ApiError::NotFound(id)
                }
                e => {
                    ApiError::Internal(format!("Failed to query entity: {}", e))
                }
            })?;

        // Set ID and mark updated
        entity.set_id(id);
        entity.mark_updated().map_err(|e| {
            ApiError::Internal(format!(
                "Failed to mark entity as updated: {}",
                e
            ))
        })?;

        // Serialize and update
        let entity_type = entity.typetag_name().to_string();
        let data_json = serde_json::to_string(&entity)?;

        let rows = txn
            .execute(
                "UPDATE entities SET data = ?1, type = ?2 WHERE id = ?3",
                params![data_json, entity_type, id as i64],
            )
            .map_err(|e| {
                ApiError::Internal(format!("Failed to update entity: {}", e))
            })?;

        if rows == 0 {
            return Err(ApiError::Conflict(
                "Entity was modified concurrently".to_string(),
            ));
        }

        txn.commit().map_err(|e| {
            ApiError::Internal(format!("Failed to commit transaction: {}", e))
        })?;

        // Return updated entity
        self.get_entity(id)?.ok_or(ApiError::NotFound(id))
    }

    fn delete_entity(&self, id: Id) -> Result<(), ApiError> {
        let mut conn = self.get_conn()?;
        let txn = conn.transaction().map_err(|e| {
            ApiError::Internal(format!("Failed to start transaction: {}", e))
        })?;

        // Delete incoming edges
        txn.execute("DELETE FROM edges WHERE dest = ?1", params![id as i64])
            .map_err(|e| {
                ApiError::Internal(format!("Failed to delete edges: {}", e))
            })?;

        // Delete outgoing edges
        txn.execute("DELETE FROM edges WHERE source = ?1", params![id as i64])
            .map_err(|e| {
                ApiError::Internal(format!("Failed to delete edges: {}", e))
            })?;

        // Delete entity
        let rows = txn
            .execute("DELETE FROM entities WHERE id = ?1", params![id as i64])
            .map_err(|e| {
                ApiError::Internal(format!("Failed to delete entity: {}", e))
            })?;

        if rows == 0 {
            return Err(ApiError::NotFound(id));
        }

        txn.commit().map_err(|e| {
            ApiError::Internal(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
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

        let mut conn = self.get_conn()?;
        let txn = ents_sqlite::Txn::new(conn.transaction().map_err(|e| {
            ApiError::Internal(format!("Failed to start transaction: {}", e))
        })?);

        audit_fn(&txn, id)
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

        let mut conn = self.get_conn()?;
        let txn = ents_sqlite::Txn::new(conn.transaction().map_err(|e| {
            ApiError::Internal(format!("Failed to start transaction: {}", e))
        })?);

        fix_fn(txn, id)
    }
}
