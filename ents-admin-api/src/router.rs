use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::backend::AdminBackend;
use crate::handlers;

/// Create an admin router for entity operations.
///
/// # Example
///
/// ```ignore
/// use ents_admin_api::admin_router;
///
/// let backend = SqliteBackend::open("app.db").unwrap();
/// let app = Router::new()
///     .nest("/admin", admin_router(backend));
/// ```
pub fn admin_router<T: AdminBackend>(backend: T) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Type discovery
        .route("/api/types", get(handlers::get_known_types::<T>))
        // Entity CRUD
        .route("/api/entities", post(handlers::create_entity::<T>))
        .route(
            "/api/entities/{id}",
            get(handlers::get_entity::<T>)
                .put(handlers::update_entity::<T>)
                .delete(handlers::delete_entity::<T>),
        )
        // Edge operations
        .route(
            "/api/entities/{id}/edges",
            get(handlers::get_entity_edges::<T>),
        )
        .route(
            "/api/entities/{id}/incoming-edges",
            get(handlers::get_incoming_edges::<T>),
        )
        // Edge audit operations
        .route(
            "/api/entities/{id}/audit-edges",
            post(handlers::audit_entity_edges::<T>),
        )
        .route(
            "/api/entities/{id}/fix-edges",
            post(handlers::fix_entity_edges::<T>),
        )
        .with_state(backend)
        .layer(cors)
}
