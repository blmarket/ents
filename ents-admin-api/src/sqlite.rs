use std::sync::Arc;

use ents::{Edge, EdgeQuery, EdgeQueryResult, Ent, Id, QueryEdge, ReadEnt};
use ents_admin::AdminEnt;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::rusqlite::params;
use r2d2_sqlite::SqliteConnectionManager;

use crate::backend::{AdminBackend, AuditResult};
use crate::error::ApiError;
use crate::type_registry::TypeRegistry;

/// Type alias for the SQLite transaction type used in the registry.
///
/// Note: The `'static` lifetime here is a marker - the actual closures in the
/// registry work with any lifetime through type coercion, as the transaction
/// type's behavior is lifetime-independent.
pub type SqliteTypeRegistry = TypeRegistry<ents_sqlite::Txn<'static>>;

/// SQLite connection pool wrapper for admin API operations.
#[derive(Clone)]
pub struct SqlitePool {
    pool: Arc<Pool<SqliteConnectionManager>>,
    type_registry: SqliteTypeRegistry,
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
    pub fn with_registry(mut self, registry: SqliteTypeRegistry) -> Self {
        self.type_registry = registry;
        self
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
    ) -> Result<EdgeQueryResult, ApiError> {
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

        // SAFETY: The audit_fn closure doesn't store any references with the
        // transaction's lifetime. The closure only uses the transaction for
        // the duration of this call, after which txn is dropped.
        let txn_static: &ents_sqlite::Txn<'static> =
            unsafe { std::mem::transmute(&txn) };
        audit_fn(txn_static, id)
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

        // SAFETY: The fix_fn closure consumes the transaction and commits it.
        // The closure doesn't store any references with the transaction's
        // lifetime beyond this call.
        let txn_static: ents_sqlite::Txn<'static> =
            unsafe { std::mem::transmute(txn) };
        fix_fn(txn_static, id)
    }

    fn list_entities(
        &self,
        entity_type: &str,
        cursor: Option<Id>,
        limit: usize,
    ) -> Result<Vec<Box<dyn Ent>>, ApiError> {
        self.with_txn(|txn| {
            txn.list_entities(entity_type, cursor, limit)
                .map_err(ApiError::from)
        })
    }

    fn known_types(&self) -> Vec<String> {
        self.type_registry
            .available_types()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}
