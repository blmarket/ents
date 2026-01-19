use axum::{
    extract::{Path, Query, State},
    Json,
};
use ents::{EdgeQuery, Ent, Id};
use serde::{Deserialize, Serialize};

use crate::backend::{AdminBackend, AuditResult};
use crate::error::{ApiError, AppJson};

#[derive(Serialize)]
pub struct EdgeResponse {
    pub source: Id,
    pub sort_key: String,
    pub sort_key_bytes: Vec<u8>,
    pub dest: Id,
}

#[derive(Serialize)]
pub struct EdgesResponse {
    pub edges: Vec<EdgeResponse>,
    pub has_more: bool,
}

#[derive(Deserialize)]
pub struct EdgesQuery {
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct TypeQuery {
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Serialize)]
pub struct CreateResponse {
    pub id: Id,
    pub entity: Box<dyn Ent>,
}

// Read operations

pub async fn get_entity<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
) -> Result<Json<Box<dyn Ent>>, ApiError> {
    let entity = backend.get_entity(id)?.ok_or(ApiError::NotFound(id))?;
    Ok(Json(entity))
}

pub async fn get_entity_edges<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
    Query(query): Query<EdgesQuery>,
) -> Result<Json<EdgesResponse>, ApiError> {
    // First verify entity exists
    let _ = backend.get_entity(id)?.ok_or(ApiError::NotFound(id))?;

    let result = if let Some(name) = &query.name {
        let name_bytes = name.as_bytes();
        let names: &[&[u8]] = &[name_bytes];
        backend.find_edges(id, EdgeQuery::asc(names))?
    } else {
        backend.find_edges(id, EdgeQuery::asc(&[]))?
    };

    let response: Vec<EdgeResponse> = result
        .edges
        .iter()
        .map(|e| EdgeResponse {
            source: e.source,
            sort_key: String::from_utf8_lossy(&e.sort_key).to_string(),
            sort_key_bytes: e.sort_key.clone(),
            dest: e.dest,
        })
        .collect();

    Ok(Json(EdgesResponse {
        edges: response,
        has_more: result.has_more,
    }))
}

pub async fn get_incoming_edges<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
) -> Result<Json<Vec<EdgeResponse>>, ApiError> {
    // First verify entity exists
    let _ = backend.get_entity(id)?.ok_or(ApiError::NotFound(id))?;

    let edges = backend.find_edges_by_dest(id)?;

    let response: Vec<EdgeResponse> = edges
        .into_iter()
        .map(|e| EdgeResponse {
            source: e.source,
            sort_key: String::from_utf8_lossy(&e.sort_key).to_string(),
            sort_key_bytes: e.sort_key,
            dest: e.dest,
        })
        .collect();

    Ok(Json(response))
}

// Write operations

pub async fn create_entity<T: AdminBackend>(
    State(backend): State<T>,
    AppJson(entity): AppJson<Box<dyn Ent>>,
) -> Result<Json<CreateResponse>, ApiError> {
    let id = backend.create_entity(entity.clone())?;
    let created = backend.get_entity(id)?.ok_or(ApiError::Internal(
        "Entity was created but could not be retrieved".to_string(),
    ))?;
    Ok(Json(CreateResponse {
        id,
        entity: created,
    }))
}

pub async fn update_entity<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
    AppJson(entity): AppJson<Box<dyn Ent>>,
) -> Result<Json<Box<dyn Ent>>, ApiError> {
    // Verify ID in path matches entity ID (if entity has non-zero ID)
    if entity.id() != 0 && entity.id() != id {
        return Err(ApiError::BadRequest(format!(
            "Entity ID {} does not match path ID {}",
            entity.id(),
            id
        )));
    }

    let updated = backend.update_entity(id, entity)?;
    Ok(Json(updated))
}

pub async fn delete_entity<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
) -> Result<Json<()>, ApiError> {
    // Verify entity exists
    let _ = backend.get_entity(id)?.ok_or(ApiError::NotFound(id))?;
    backend.delete_entity(id)?;
    Ok(Json(()))
}

// Type discovery

pub async fn get_known_types<T: AdminBackend>(
    State(backend): State<T>,
) -> Json<Vec<String>> {
    Json(backend.known_types())
}

// Edge audit operations

pub async fn audit_entity_edges<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
    Query(query): Query<TypeQuery>,
) -> Result<Json<AuditResult>, ApiError> {
    let result = backend.audit_entity_edges(id, &query.type_name)?;
    Ok(Json(result))
}

pub async fn fix_entity_edges<T: AdminBackend>(
    State(backend): State<T>,
    Path(id): Path<Id>,
    Query(query): Query<TypeQuery>,
) -> Result<Json<()>, ApiError> {
    backend.fix_entity_edges(id, &query.type_name)?;
    Ok(Json(()))
}

#[derive(Deserialize)]
pub struct ListEntitiesQuery {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub cursor: Option<Id>,
    pub limit: Option<usize>,
}

pub async fn list_entities<T: AdminBackend>(
    State(backend): State<T>,
    Query(query): Query<ListEntitiesQuery>,
) -> Result<Json<Vec<Box<dyn Ent>>>, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let entities =
        backend.list_entities(&query.entity_type, query.cursor, limit)?;
    Ok(Json(entities))
}
