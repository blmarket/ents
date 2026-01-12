mod backend;
mod error;
mod handlers;
mod router;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use backend::{AdminBackend, AuditResult, EdgeInfo};
pub use error::ApiError;
pub use router::admin_router;
