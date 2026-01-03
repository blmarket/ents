use std::borrow::BorrowMut;

use ents::Edge;
use ents::{
    DatabaseError, EdgeDraft, EdgeProvider, EdgeQuery, EdgeValue, Ent, EntWithEdges, Id, QueryEdge,
    SortOrder, Transactional,
};
use r2d2_sqlite::rusqlite::{params, OptionalExtension, Transaction};

pub struct Txn<'conn>(Transaction<'conn>);

impl<'conn> Txn<'conn> {
    pub fn new(tx: Transaction<'conn>) -> Self {
        Self(tx)
    }

    fn update(
        &self,
        id: Id,
        ent: Box<dyn Ent>,
        expected_last_updated: Option<u64>,
    ) -> Result<bool, DatabaseError> {
        // Serialize the entity to JSON
        let entity_type = ent.typetag_name().to_string();
        let data_json = serde_json::to_string(&ent).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        // Build the UPDATE query with optional CAS check
        let rows_affected = self
            .0
            .execute(
                r#"
                UPDATE entities SET data = ?1, type = ?2
                WHERE
                    id = ?3 AND
                    (
                        JSON_EXTRACT(data, '$.last_updated') = ?4 OR
                        ?4 IS NULL
                    )
                "#,
                params![data_json, entity_type, id as i64, expected_last_updated],
            )
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        Ok(rows_affected > 0)
    }
}

impl<'conn> Txn<'conn> {
    fn insert<E: Ent>(&self, ent: &E) -> Result<Id, DatabaseError> {
        // Serialize the entity to JSON
        let entity_type = ent.typetag_name().to_string();

        // Had to cast to &dyn Ent to make sure `type` to be serialized as well.
        let data_json =
            serde_json::to_string(&(ent as &dyn Ent)).map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        self.0
            .execute(
                "INSERT INTO entities (type, data) VALUES (?1, ?2)",
                params![entity_type, data_json],
            )
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        let inserted_id = self.0.last_insert_rowid() as Id;

        Ok(inserted_id)
    }
}

impl<'conn> Transactional for Txn<'conn> {
    fn get(&self, id: Id) -> Result<Option<Box<dyn Ent>>, DatabaseError> {
        let mut stmt = self
            .0
            .prepare("SELECT id, data FROM entities WHERE id = ?1")
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        stmt.query_row(params![id as i64], |row| {
            let id: Id = row.get(0)?;
            let data_json: &str = row.get_ref(1)?.as_str()?;
            let mut ret =
                serde_json::from_str::<Box<dyn Ent>>(data_json).expect("failed to parse JSON");
            ret.set_id(id);
            Ok(ret)
        })
        .optional()
        .map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })
    }

    fn create_edge(&self, edge: EdgeValue) -> Result<(), DatabaseError> {
        let source = edge.source;
        let sort_key = edge.sort_key;
        let dest = edge.dest;

        self.0
            .execute(
                "INSERT INTO edges (source, type, dest) VALUES (?1, ?2, ?3)",
                params![source as i64, sort_key, dest as i64],
            )
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        Ok(())
    }

    fn delete<E: Ent + EntWithEdges>(&self, id: Id) -> Result<(), DatabaseError> {
        self.0
            .prepare_cached(
                r#"
        DELETE FROM edges WHERE dest = ?1;
        "#,
            )
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?
            .execute(params![id])
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        self.0
            .prepare_cached(
                r#"
        DELETE FROM entities WHERE id = ?1;
        "#,
            )
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?
            .execute(params![id])
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        Ok(())
    }

    fn update<T: EntWithEdges, F: FnOnce(&mut T), B: BorrowMut<T>>(
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
            return self.update(
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

        let updated = self.update(
            ent.id(),
            dyn_clone::clone_box(ent),
            Some(expected_last_updated),
        )?;

        if updated {
            // Remove old edges if they existed
            for edge in edge0 {
                self.0
                    .execute(
                        "DELETE FROM edges WHERE source = ?1 AND type = ?2 AND dest = ?3",
                        params![edge.source as i64, edge.sort_key, edge.dest as i64],
                    )
                    .map_err(|e| DatabaseError::Other {
                        source: Box::new(e),
                    })?;
            }

            // Create new edges if they exist
            for edge in edge1 {
                self.create_edge(edge)?;
            }
        }

        Ok(updated)
    }

    fn create<E: Ent + EntWithEdges>(&self, mut ent: E) -> Result<Id, DatabaseError> {
        let id = self.insert(&ent)?;
        ent.set_id(id);
        ent.setup_edges(self).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;
        Ok(id)
    }

    fn commit(self) -> Result<(), DatabaseError> {
        self.0.commit().map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })
    }
}

impl<'conn> QueryEdge for Txn<'conn> {
    fn find_edges(&self, source: Id, query: EdgeQuery) -> Result<Vec<Edge>, DatabaseError> {
        // Build WHERE clause for edge names filter
        let name_filter = if query.edge_names.is_empty() {
            String::new()
        } else {
            let placeholders = query
                .edge_names
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            format!(" AND type IN ({})", placeholders)
        };

        // Build cursor filter based on sort order
        let cursor_filter = match (&query.cursor, query.order) {
            (Some(_), SortOrder::Asc) => " AND (type, dest) > (?, ?)",
            (Some(_), SortOrder::Desc) => " AND (type, dest) < (?, ?)",
            (None, _) => "",
        };

        // Build ORDER BY clause
        let order_clause = match query.order {
            SortOrder::Asc => "ORDER BY type ASC, dest ASC",
            SortOrder::Desc => "ORDER BY type DESC, dest DESC",
        };

        let sql = format!(
            "SELECT source, type, dest FROM edges WHERE source = ?{}{} {} LIMIT 100",
            name_filter, cursor_filter, order_clause
        );

        // Build parameters
        let mut params: Vec<Box<dyn r2d2_sqlite::rusqlite::ToSql>> = Vec::new();
        params.push(Box::new(source));

        for name in query.edge_names {
            params.push(Box::new(name.to_vec()));
        }

        if let Some(cursor) = query.cursor {
            params.push(Box::new(cursor.sort_key.to_vec()));
            params.push(Box::new(cursor.destination));
        }

        let params_refs: Vec<&dyn r2d2_sqlite::rusqlite::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.0.prepare(&sql).map_err(|e| DatabaseError::Other {
            source: Box::new(e),
        })?;

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let source: i64 = row.get(0)?;
                let sort_key: Vec<u8> = match row.get_ref(1)? {
                    r2d2_sqlite::rusqlite::types::ValueRef::Text(s) => s.to_vec(),
                    r2d2_sqlite::rusqlite::types::ValueRef::Blob(b) => b.to_vec(),
                    _ => {
                        return Err(r2d2_sqlite::rusqlite::Error::InvalidColumnType(
                            1,
                            "type".into(),
                            row.get_ref(1)?.data_type(),
                        ))
                    }
                };
                let destination: i64 = row.get(2)?;
                Ok(Edge::new(source as Id, sort_key, destination as Id))
            })
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| DatabaseError::Other {
                source: Box::new(e),
            })
    }
}
