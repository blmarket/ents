mod edge_provider;
mod query_edge;

use std::{any::Any, borrow::BorrowMut};

pub use edge_provider::{
    DraftError, EdgeDraft, EdgeValue, IncomingEdgeProvider, NullEdgeDraft,
    NullEdgeProvider,
};
pub use query_edge::{
    Edge, EdgeCursor, EdgeQuery, EdgeQueryResult, QueryEdge, SortOrder,
};

/// Unique identifier for an entity
pub type Id = u64;

pub trait ReadEnt: QueryEdge {
    fn get(&self, id: Id) -> Result<Option<Box<dyn Ent>>, DatabaseError>;
}

pub trait Transactional: ReadEnt {
    fn create<E: Ent>(&self, ent: E) -> Result<Id, DatabaseError>;

    fn delete(&self, id: Id) -> Result<(), DatabaseError>;

    fn create_edge(&self, edge: EdgeValue) -> Result<(), DatabaseError>;

    fn update<T, F, B>(
        &self,
        ent: B,
        mutator: F,
    ) -> Result<bool, DatabaseError>
    where
        T: Ent,
        F: FnOnce(&mut T),
        B: BorrowMut<T>;

    fn commit(self) -> Result<(), DatabaseError>;
}

pub trait TransactionProvider: 'static + Clone {
    type Tx<'a>: Transactional;

    fn execute<R, F>(&self, func: F) -> Result<R, DatabaseError>
    where
        F: for<'a> FnOnce(Self::Tx<'a>) -> R;
}

/// Error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Entity capacity reached")]
    EntCapacityReached,
    #[error("Other error: {source}")]
    Other {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// Error type for entity mutations
#[derive(Debug, thiserror::Error)]
pub enum EntMutationError {
    #[error("Other error: {0}")]
    Other(String),
}

#[typetag::serde(tag = "type")]
pub trait Ent: Any + dyn_clone::DynClone + Send + Sync {
    type EdgeProvider: IncomingEdgeProvider<Self>
    where
        Self: Sized;

    fn id(&self) -> Id;
    fn set_id(&mut self, id: Id);
    fn last_updated(&self) -> u64;
    fn mark_updated(&mut self) -> Result<(), EntMutationError>;

    fn setup_edges<T: Transactional>(&self, txn: &T) -> Result<(), DraftError>
    where
        Self: Sized,
    {
        let draft = Self::EdgeProvider::draft(self);
        for edge in draft.check(txn)? {
            txn.create_edge(edge)?;
        }
        Ok(())
    }
}

dyn_clone::clone_trait_object!(Ent);

pub trait EntExt {
    fn is<T: Ent>(&self) -> bool;

    fn as_ent<T: Ent>(&self) -> Option<&T>;

    fn as_ent_mut<T: Ent>(&mut self) -> Option<&mut T>;

    fn downcast_ent<T: Ent>(self) -> Option<Box<T>>;

    fn into_ent<T: Ent>(self) -> Option<T>
    where
        Self: Sized,
    {
        self.downcast_ent().map(|x| *x)
    }
}

impl EntExt for Box<dyn Ent> {
    fn is<T: Ent>(&self) -> bool {
        (&**self as &dyn Any).is::<T>()
    }

    fn as_ent<T: Ent>(&self) -> Option<&T> {
        (&**self as &dyn Any).downcast_ref::<T>()
    }

    fn as_ent_mut<T: Ent>(&mut self) -> Option<&mut T> {
        (&mut **self as &mut dyn Any).downcast_mut::<T>()
    }

    fn downcast_ent<T: Ent>(self) -> Option<Box<T>> {
        (self as Box<dyn Any>).downcast::<T>().ok()
    }
}
