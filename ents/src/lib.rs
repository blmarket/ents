pub mod edge_provider;
pub mod query_edge;

use std::any::Any;

pub use edge_provider::{
    DraftError, EdgeDraft, EdgeProvider, EdgeValue, EntWithEdges,
    NullEdgeDraft, NullEdgeProvider, Transactional,
};
pub use query_edge::{Edge, EdgeCursor, EdgeQuery, QueryEdge, SortOrder};

/// Unique identifier for an entity
pub type Id = u64;

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
    fn id(&self) -> Id;
    fn set_id(&mut self, id: Id);
    fn last_updated(&self) -> u64;
    fn mark_updated(&mut self) -> Result<(), EntMutationError>;
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
