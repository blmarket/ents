mod backend;
mod error;
mod handlers;
mod router;
mod type_registry;

pub use type_registry::{TypeRegistry, TypeRegistryBuilder};

pub use backend::{AdminBackend, AdminBackendProvider, AuditResult, EdgeInfo};
pub use error::{ApiError, AppJson};
pub use router::admin_router;
