use ents::{
    DatabaseError, DraftError, Edge, EdgeDraft, EdgeValue, Ent, EntExt, Id,
    IncomingEdgeProvider, Transactional,
};

/// Error type for edge audit operations.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Entity not found: {0}")]
    EntityNotFound(Id),

    #[error("Unexpected entity type: {0} is not {1} type")]
    UnexpectedEntityType(Id, String),

    #[error("Edge mismatch: existing edges differ from drafted edges")]
    EdgeMismatch {
        existing: Vec<EdgeValue>,
        drafted: Vec<EdgeValue>,
    },

    #[error("Draft error: {0}")]
    Draft(#[from] DraftError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
}

pub trait AdminEnt: Transactional {
    fn find_edges_by_dest(&self, dest: Id) -> Result<Vec<Edge>, DatabaseError>;

    fn remove_edges_by_dest(&self, dest: Id) -> Result<(), DatabaseError>;

    fn audit_ent_edges<E: Ent>(&self, id: Id) -> Result<(), AuditError>
    where
        Self: Sized,
    {
        // Step 1: Get the entity and verify it exists and is of correct type
        let ent_box = self.get(id)?.ok_or(AuditError::EntityNotFound(id))?;

        let ent = ent_box.as_ent::<E>().ok_or_else(|| {
            AuditError::UnexpectedEntityType(
                id,
                std::any::type_name::<E>().to_string(),
            )
        })?;

        // Step 2: Find all edges whose dest is entity id
        let mut existing_edges: Vec<EdgeValue> = self
            .find_edges_by_dest(id)?
            .into_iter()
            .map(|e| EdgeValue::new(e.source, e.sort_key, e.dest))
            .collect();
        existing_edges.sort_by(|a, b| {
            (&a.source, &a.sort_key).cmp(&(&b.source, &b.sort_key))
        });

        // Step 3: Remove all edges from step 1
        self.remove_edges_by_dest(id)?;

        // Step 4: Draft edges (this validates and creates new edge values)
        let draft = E::EdgeProvider::draft(ent);
        let mut drafted_edges = draft.check(self)?;
        drafted_edges.sort_by(|a, b| {
            (&a.source, &a.sort_key).cmp(&(&b.source, &b.sort_key))
        });

        // Step 5: Check edge vector is same with one from step 1
        if existing_edges != drafted_edges {
            return Err(AuditError::EdgeMismatch {
                existing: existing_edges,
                drafted: drafted_edges,
            });
        }

        // Step 6: Transaction will be dropped (not committed) - database unchanged
        // The caller is responsible for not committing this transaction
        Ok(())
    }

    fn fix_ent_edges<E: Ent>(self, id: Id) -> Result<(), AuditError>
    where
        Self: Sized,
    {
        // Step 1: Get the entity and verify it exists and is of correct type
        let ent_box = self.get(id)?.ok_or(AuditError::EntityNotFound(id))?;

        let ent = ent_box.as_ent::<E>().ok_or_else(|| {
            AuditError::UnexpectedEntityType(
                id,
                std::any::type_name::<E>().to_string(),
            )
        })?;

        // Step 2: Remove all incoming edges
        self.remove_edges_by_dest(id)?;

        // Step 3: Draft and create new edges
        let draft = E::EdgeProvider::draft(ent);
        let edges = draft.check(&self)?;

        for edge in edges {
            self.create_edge(edge)?;
        }

        self.commit()?;

        Ok(())
    }

    /// List entities of a specific type with cursor-based pagination.
    ///
    /// # Arguments
    /// * `entity_type` - The string name of the entity type to list
    /// * `cursor` - Optional ID cursor. If provided, returns entities with ID > cursor
    /// * `limit` - Maximum number of entities to return
    fn list_entities(
        &self,
        entity_type: &str,
        cursor: Option<Id>,
        limit: usize,
    ) -> Result<Vec<Box<dyn Ent>>, DatabaseError>;
}
