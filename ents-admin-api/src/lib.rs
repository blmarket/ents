mod backend;
mod error;
mod handlers;
mod router;
mod type_registry;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteTypeRegistry;

pub use type_registry::{TypeRegistry, TypeRegistryBuilder};

pub use backend::{AdminBackend, AuditResult, EdgeInfo};
pub use error::ApiError;
pub use router::admin_router;
