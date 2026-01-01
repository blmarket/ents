use crate::{DatabaseError, Id};

/// Sort order for edge queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Ascending order (smallest to largest)
    Asc,
    /// Descending order (largest to smallest)
    Desc,
}

/// Cursor for pagination combining sort key and destination
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeCursor<'a> {
    /// The sort key value at the cursor position
    pub sort_key: &'a [u8],
    /// The destination ID at the cursor position
    pub destination: Id,
}

impl<'a> EdgeCursor<'a> {
    /// Create a new cursor
    pub fn new(sort_key: &'a [u8], destination: Id) -> Self {
        Self {
            sort_key,
            destination,
        }
    }
}

/// Edge result containing all three properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// Source entity ID
    pub source: Id,
    /// Sort key for ordering
    pub sort_key: Vec<u8>,
    /// Destination entity ID
    pub dest: Id,
}

impl Edge {
    /// Create a new edge
    pub fn new(source: Id, sort_key: Vec<u8>, dest: Id) -> Self {
        Self {
            source,
            sort_key,
            dest,
        }
    }
}

/// Query parameters for edge enumeration
#[derive(Debug, Clone)]
pub struct EdgeQuery<'a> {
    /// Filter edges by name (IN clause). If empty, no name filtering is applied.
    pub edge_names: &'a [&'a [u8]],
    /// Sort order for results
    pub order: SortOrder,
    /// Cursor for pagination:
    /// - For Asc order: returns edges with (sort_key, destination) > cursor
    /// - For Desc order: returns edges with (sort_key, destination) < cursor
    pub cursor: Option<EdgeCursor<'a>>,
}

impl<'a> EdgeQuery<'a> {
    /// Create a new query with ascending order
    pub fn asc(edge_names: &'a [&'a [u8]]) -> Self {
        Self {
            edge_names,
            order: SortOrder::Asc,
            cursor: None,
        }
    }

    /// Create a new query with descending order
    pub fn desc(edge_names: &'a [&'a [u8]]) -> Self {
        Self {
            edge_names,
            order: SortOrder::Desc,
            cursor: None,
        }
    }

    /// Set the pagination cursor
    pub fn with_cursor(mut self, cursor: EdgeCursor<'a>) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_cursor_opt(mut self, cursor: Option<EdgeCursor<'a>>) -> Self {
        self.cursor = cursor;
        self
    }
}

pub trait QueryEdge {
    /// Find edges with flexible filtering and ordering options.
    ///
    /// # Arguments
    /// * `source` - The source entity ID
    /// * `query` - Query parameters specifying filters, ordering, and pagination
    ///
    /// Returns up to 100 edges matching the query criteria, sorted by (sort_key, destination).
    /// For ascending order, edges are returned where (sort_key, destination) > cursor.
    /// For descending order, edges are returned where (sort_key, destination) < cursor.
    fn find_edges(&self, source: Id, query: EdgeQuery) -> Result<Vec<Edge>, DatabaseError>;
}
