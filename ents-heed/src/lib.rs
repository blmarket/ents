//! LMDB-based entity storage implementation using the heed crate.
//!
//! This module provides an LMDB (via heed) implementation of the entity storage traits,
//! mirroring the functionality of ents-sqlite but using LMDB as the underlying store.
//!
//! # Storage Layout
//!
//! The implementation uses three LMDB databases:
//! - `entities`: Maps entity IDs to serialized entity JSON
//! - `edges`: Maps composite keys (source, sort_key, dest) to empty values
//! - `meta`: Stores metadata like the next entity ID

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use byteorder::{BigEndian, ByteOrder};
use ents::{
    DatabaseError, Edge, EdgeDraft, EdgeProvider, EdgeQuery, EdgeValue, Ent, EntWithEdges, Id,
    QueryEdge, SortOrder, Transactional,
};
use heed::types::{Bytes, Str};
use heed::{Database, Env, EnvOpenOptions, RwTxn};
use snowflaked::Generator;

/// Maximum number of edges returned by find_edges
const MAX_EDGES: usize = 100;

/// LMDB environment wrapper that manages the databases.
pub struct HeedEnv {
    env: Env,
    entities: Database<heed::types::U64<BigEndian>, Str>,
    edges: Database<Bytes, Bytes>,
    id_generator: Mutex<Generator>,
}

impl HeedEnv {
    /// Opens or creates an LMDB environment at the given path.
    ///
    /// # Arguments
    /// * `path` - Directory path for the LMDB environment
    /// * `map_size` - Maximum size of the database in bytes (default: 1GB)
    pub fn open<P: AsRef<Path>>(path: P, map_size: Option<usize>) -> Result<Self, DatabaseError> {
        let path = path.as_ref();
        fs::create_dir_all(path).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(map_size.unwrap_or(1024 * 1024 * 1024)) // 1GB default
                .max_dbs(2)
                .open(path)
        }
        .map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        // Create or open the databases
        let mut wtxn = env.write_txn().map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        let entities: Database<heed::types::U64<BigEndian>, Str> = env
            .create_database(&mut wtxn, Some("entities"))
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        let edges: Database<Bytes, Bytes> =
            env.create_database(&mut wtxn, Some("edges"))
                .map_err(|e| DatabaseError::Other {
                    source: Box::new(e),
                })?;

        wtxn.commit().map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        // Initialize snowflake ID generator
        // Using node_id 0, can be configured if needed for distributed systems
        let id_generator = Generator::new(0);

        Ok(Self {
            env,
            entities,
            edges,
            id_generator: Mutex::new(id_generator),
        })
    }

    /// Begins a read-write transaction.
    pub fn write_txn(&self) -> Result<Txn<'_>, DatabaseError> {
        let txn = self.env.write_txn().map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;
        Ok(Txn {
            txn: RefCell::new(txn),
            env: self,
        })
    }

    /// Allocates the next entity ID using snowflake algorithm.
    fn next_id(&self) -> Result<Id, DatabaseError> {
        let mut generator = self.id_generator.lock().map_err(|e| DatabaseError::Other {
            source: Box::new(std::io::Error::other(format!(
                "Failed to lock ID generator: {}",
                e
            ))),
        })?;
        Ok(generator.generate())
    }
}

/// A read-write transaction wrapper.
///
/// Uses interior mutability via RefCell to satisfy the Transactional trait's
/// requirement for &self methods while still allowing mutation.
pub struct Txn<'env> {
    txn: RefCell<RwTxn<'env>>,
    env: &'env HeedEnv,
}

impl<'env> Txn<'env> {
    /// Inserts an entity and returns its assigned ID.
    fn insert<E: Ent>(&self, ent: &E) -> Result<Id, DatabaseError> {
        let id = self.env.next_id()?;
        let mut wtxn = self.txn.borrow_mut();

        let data_json =
            serde_json::to_string(&(ent as &dyn Ent)).map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        self.env
            .entities
            .put(&mut wtxn, &id, &data_json)
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        Ok(id)
    }

    /// Internal update that writes entity with optional CAS check.
    fn update_internal(
        &self,
        id: Id,
        ent: Box<dyn Ent>,
        expected_last_updated: Option<u64>,
    ) -> Result<bool, DatabaseError> {
        // If CAS check is needed, verify current last_updated
        if let Some(expected) = expected_last_updated {
            if let Some(current) = self.get(id)? {
                if current.last_updated() != expected {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }

        let data_json = serde_json::to_string(&ent).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        self.env
            .entities
            .put(&mut self.txn.borrow_mut(), &id, &data_json)
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        Ok(true)
    }

    fn delete_edge(&self, source: Id, sort_key: &[u8], dest: Id) -> Result<(), DatabaseError> {
        let key = make_edge_key(source, sort_key, dest);
        self.env
            .edges
            .delete(&mut self.txn.borrow_mut(), &key)
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;
        Ok(())
    }
}

impl<'env> Transactional for Txn<'env> {
    fn get(&self, id: Id) -> Result<Option<Box<dyn Ent>>, DatabaseError> {
        let txn = self.txn.borrow();
        match self
            .env
            .entities
            .get(&txn, &id)
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })? {
            Some(data_json) => {
                let mut ent = serde_json::from_str::<Box<dyn Ent>>(data_json).map_err(|e| {
                    DatabaseError::Other {
                        source: Box::new(e),
                    }
                })?;
                ent.set_id(id);
                Ok(Some(ent))
            }
            None => Ok(None),
        }
    }

    fn create<E: Ent + EntWithEdges>(&self, mut ent: E) -> Result<Id, DatabaseError> {
        let id = self.insert(&ent)?;
        ent.set_id(id);
        ent.setup_edges(self).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;
        Ok(id)
    }

    fn delete<E: Ent + EntWithEdges>(&self, id: Id) -> Result<(), DatabaseError> {
        // Delete edges where this entity is the destination
        // We need to scan all edges and delete matching ones
        let to_delete: Vec<Vec<u8>> = {
            let txn = self.txn.borrow();
            let iter = self
                .env
                .edges
                .iter(&txn)
                .map_err(|e| DatabaseError::Other {
                    source: Box::new(e),
                })?;

            let mut keys = Vec::new();
            for result in iter {
                let (key, _) = result.map_err(|e| DatabaseError::Other {
                    source: Box::new(e),
                })?;
                let (_, _, dest) = parse_edge_key(key);
                if dest == id {
                    keys.push(key.to_vec());
                }
            }
            keys
        };

        for key in to_delete {
            self.env
                .edges
                .delete(&mut self.txn.borrow_mut(), &key)
                .map_err(|e| DatabaseError::Other {
                    source: Box::new(e),
                })?;
        }

        // Delete the entity
        self.env
            .entities
            .delete(&mut self.txn.borrow_mut(), &id)
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        Ok(())
    }

    fn create_edge(&self, edge: EdgeValue) -> Result<(), DatabaseError> {
        let key = make_edge_key(edge.source, &edge.sort_key, edge.dest);
        self.env
            .edges
            .put(&mut self.txn.borrow_mut(), &key, &[])
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;
        Ok(())
    }

    fn update<T: Ent + EntWithEdges + 'static, F: FnOnce(&mut T), B: BorrowMut<T>>(
        &self,
        mut ent0: B,
        mutator: F,
    ) -> Result<bool, DatabaseError> {
        let ent = ent0.borrow_mut();
        let draft0 = T::EdgeProvider::draft(ent);
        let expected_last_updated = ent.last_updated();

        mutator(ent);
        ent.mark_updated().map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        let draft1 = T::EdgeProvider::draft(ent);

        // Optimization: if drafts are equal, no edge changes needed
        if draft0 == draft1 {
            return self.update_internal(
                ent.id(),
                dyn_clone::clone_box(ent),
                Some(expected_last_updated),
            );
        }

        let edge0 = draft0.check(self).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;
        let edge1 = draft1.check(self).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        let updated = self.update_internal(
            ent.id(),
            dyn_clone::clone_box(ent),
            Some(expected_last_updated),
        )?;

        if updated {
            // Remove old edges if they existed
            for edge in edge0 {
                self.delete_edge(edge.source, &edge.sort_key, edge.dest)?;
            }

            // Create new edges if they exist
            for edge in edge1 {
                self.create_edge(edge)?;
            }
        }

        Ok(updated)
    }

    fn commit(self) -> Result<(), DatabaseError> {
        self.txn
            .into_inner()
            .commit()
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })
    }
}

impl<'env> QueryEdge for Txn<'env> {
    fn find_edges(&self, source: Id, query: EdgeQuery) -> Result<Vec<Edge>, DatabaseError> {
        let txn = self.txn.borrow();
        find_edges_internal(&txn, &self.env.edges, source, query)
    }
}

/// Creates a composite key for an edge: source (8 bytes) + sort_key + dest (8 bytes)
fn make_edge_key(source: Id, sort_key: &[u8], dest: Id) -> Vec<u8> {
    let mut key = Vec::with_capacity(8 + sort_key.len() + 8);
    let mut buf = [0u8; 8];

    BigEndian::write_u64(&mut buf, source);
    key.extend_from_slice(&buf);

    key.extend_from_slice(sort_key);

    BigEndian::write_u64(&mut buf, dest);
    key.extend_from_slice(&buf);

    key
}

/// Parses a composite edge key into (source, sort_key, dest)
fn parse_edge_key(key: &[u8]) -> (Id, &[u8], Id) {
    let source = BigEndian::read_u64(&key[0..8]);
    let dest = BigEndian::read_u64(&key[key.len() - 8..]);
    let sort_key = &key[8..key.len() - 8];
    (source, sort_key, dest)
}

fn find_edges_internal(
    txn: &heed::RoTxn<'_>,
    edges_db: &Database<Bytes, Bytes>,
    source: Id,
    query: EdgeQuery,
) -> Result<Vec<Edge>, DatabaseError> {
    let mut results = Vec::new();

    // Create the prefix for this source
    let mut prefix = [0u8; 8];
    BigEndian::write_u64(&mut prefix, source);

    // Get iterator
    let iter = edges_db
        .prefix_iter(txn, &prefix)
        .map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

    // Collect all matching edges
    let mut all_edges: Vec<Edge> = Vec::new();

    for result in iter {
        let (key, _) = result.map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        let (src, sort_key, dest) = parse_edge_key(key);
        if src != source {
            break; // Past our prefix
        }

        // Apply edge name filter if specified
        if !query.edge_names.is_empty() && !query.edge_names.contains(&sort_key) {
            continue;
        }

        all_edges.push(Edge::new(src, sort_key.to_vec(), dest));
    }

    // Sort based on order
    match query.order {
        SortOrder::Asc => {
            all_edges.sort_by(|a, b| (&a.sort_key, a.dest).cmp(&(&b.sort_key, b.dest)));
        }
        SortOrder::Desc => {
            all_edges.sort_by(|a, b| (&b.sort_key, b.dest).cmp(&(&a.sort_key, a.dest)));
        }
    }

    // Apply cursor filter
    for edge in all_edges {
        if let Some(ref cursor) = query.cursor {
            let edge_key = (edge.sort_key.as_slice(), edge.dest);
            let cursor_key = (cursor.sort_key, cursor.destination);

            match query.order {
                SortOrder::Asc => {
                    if edge_key <= cursor_key {
                        continue;
                    }
                }
                SortOrder::Desc => {
                    if edge_key >= cursor_key {
                        continue;
                    }
                }
            }
        }

        results.push(edge);

        if results.len() >= MAX_EDGES {
            break;
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_key_roundtrip() {
        let source = 12345u64;
        let sort_key = b"test_edge";
        let dest = 67890u64;

        let key = make_edge_key(source, sort_key, dest);
        let (parsed_source, parsed_sort_key, parsed_dest) = parse_edge_key(&key);

        assert_eq!(parsed_source, source);
        assert_eq!(parsed_sort_key, sort_key);
        assert_eq!(parsed_dest, dest);
    }

    #[test]
    fn test_edge_key_ordering() {
        // Verify that keys sort correctly
        let key1 = make_edge_key(1, b"a", 10);
        let key2 = make_edge_key(1, b"a", 20);
        let key3 = make_edge_key(1, b"b", 10);
        let key4 = make_edge_key(2, b"a", 10);

        assert!(key1 < key2); // Same source and type, different dest
        assert!(key2 < key3); // Same source, different type
        assert!(key3 < key4); // Different source
    }
}
