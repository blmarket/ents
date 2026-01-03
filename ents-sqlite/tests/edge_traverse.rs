use ents::{
    DatabaseError, Edge, EdgeCursor, EdgeQuery, EdgeValue, Id, QueryEdge,
    Transactional,
};
use ents_sqlite::Txn;
use r2d2_sqlite::rusqlite::Connection;

/// Helper to create an in-memory database with required schema
fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute(
        "CREATE TABLE entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            data TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE edges (
            source INTEGER NOT NULL,
            type BLOB NOT NULL,
            dest INTEGER NOT NULL,
            PRIMARY KEY (source, type, dest)
        )",
        [],
    )
    .unwrap();

    conn
}

/// Helper to insert test edges
fn insert_edges(
    txn: &Txn,
    edges: &[(Id, &[u8], Id)],
) -> Result<(), DatabaseError> {
    for (source, sort_key, dest) in edges {
        txn.create_edge(EdgeValue {
            source: *source,
            sort_key: sort_key.to_vec(),
            dest: *dest,
        })?;
    }
    Ok(())
}

#[test]
fn test_find_edges_asc_no_filter() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    // Insert test edges with different types
    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"likes", 5),
            (1, b"likes", 15),
            (1, b"blocks", 30),
        ],
    )
    .unwrap();

    // Query all edges in ascending order
    let query = EdgeQuery::asc(&[]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 5);

    // Verify ascending order by (type, dest)
    assert_eq!(result[0], Edge::new(1, b"blocks".to_vec(), 30));
    assert_eq!(result[1], Edge::new(1, b"follows".to_vec(), 10));
    assert_eq!(result[2], Edge::new(1, b"follows".to_vec(), 20));
    assert_eq!(result[3], Edge::new(1, b"likes".to_vec(), 5));
    assert_eq!(result[4], Edge::new(1, b"likes".to_vec(), 15));
}

#[test]
fn test_find_edges_desc_no_filter() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"likes", 5),
            (1, b"likes", 15),
            (1, b"blocks", 30),
        ],
    )
    .unwrap();

    // Query all edges in descending order
    let query = EdgeQuery::desc(&[]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 5);

    // Verify descending order by (type, dest)
    assert_eq!(result[0], Edge::new(1, b"likes".to_vec(), 15));
    assert_eq!(result[1], Edge::new(1, b"likes".to_vec(), 5));
    assert_eq!(result[2], Edge::new(1, b"follows".to_vec(), 20));
    assert_eq!(result[3], Edge::new(1, b"follows".to_vec(), 10));
    assert_eq!(result[4], Edge::new(1, b"blocks".to_vec(), 30));
}

#[test]
fn test_find_edges_with_single_name_filter() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"likes", 5),
            (1, b"likes", 15),
        ],
    )
    .unwrap();

    // Query only "follows" edges
    let query = EdgeQuery::asc(&[b"follows"]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Edge::new(1, b"follows".to_vec(), 10));
    assert_eq!(result[1], Edge::new(1, b"follows".to_vec(), 20));
}

#[test]
fn test_find_edges_with_multiple_name_filters() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"likes", 5),
            (1, b"likes", 15),
            (1, b"blocks", 30),
        ],
    )
    .unwrap();

    // Query "follows" and "likes" edges
    let query = EdgeQuery::asc(&[b"follows" as &[u8], b"likes" as &[u8]]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 4);
    assert_eq!(result[0], Edge::new(1, b"follows".to_vec(), 10));
    assert_eq!(result[1], Edge::new(1, b"follows".to_vec(), 20));
    assert_eq!(result[2], Edge::new(1, b"likes".to_vec(), 5));
    assert_eq!(result[3], Edge::new(1, b"likes".to_vec(), 15));
}

#[test]
fn test_find_edges_asc_with_cursor() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"follows", 30),
            (1, b"likes", 5),
            (1, b"likes", 15),
        ],
    )
    .unwrap();

    // Query with cursor after ("follows", 10)
    let cursor = EdgeCursor::new(b"follows", 10);
    let query = EdgeQuery::asc(&[]).with_cursor(cursor);
    let result = txn.find_edges(1, query).unwrap();

    // Should return edges after ("follows", 10)
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], Edge::new(1, b"follows".to_vec(), 20));
    assert_eq!(result[1], Edge::new(1, b"follows".to_vec(), 30));
    assert_eq!(result[2], Edge::new(1, b"likes".to_vec(), 5));
    assert_eq!(result[3], Edge::new(1, b"likes".to_vec(), 15));
}

#[test]
fn test_find_edges_desc_with_cursor() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"follows", 30),
            (1, b"likes", 5),
            (1, b"likes", 15),
        ],
    )
    .unwrap();

    // Query with cursor before ("likes", 5) in descending order
    let cursor = EdgeCursor::new(b"likes", 5);
    let query = EdgeQuery::desc(&[]).with_cursor(cursor);
    let result = txn.find_edges(1, query).unwrap();

    // Should return edges before ("likes", 5) in descending order
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], Edge::new(1, b"follows".to_vec(), 30));
    assert_eq!(result[1], Edge::new(1, b"follows".to_vec(), 20));
    assert_eq!(result[2], Edge::new(1, b"follows".to_vec(), 10));
}

#[test]
fn test_find_edges_asc_cursor_with_filter() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (1, b"follows", 30),
            (1, b"likes", 5),
            (1, b"likes", 15),
        ],
    )
    .unwrap();

    // Query "follows" edges with cursor after ("follows", 10)
    let cursor = EdgeCursor::new(b"follows", 10);
    let query = EdgeQuery::asc(&[b"follows"]).with_cursor(cursor);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Edge::new(1, b"follows".to_vec(), 20));
    assert_eq!(result[1], Edge::new(1, b"follows".to_vec(), 30));
}

#[test]
fn test_find_edges_empty_result() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(&txn, &[(1, b"follows", 10), (1, b"follows", 20)]).unwrap();

    // Query for non-existent edge type
    let query = EdgeQuery::asc(&[b"blocks"]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_edges_no_edges_for_source() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(&txn, &[(1, b"follows", 10)]).unwrap();

    // Query for different source
    let query = EdgeQuery::asc(&[]);
    let result = txn.find_edges(999, query).unwrap();

    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_edges_pagination_asc() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    // Insert many edges
    let mut edges = Vec::new();
    for i in 1..=10 {
        edges.push((1, b"item" as &[u8], i * 10));
    }
    insert_edges(&txn, &edges).unwrap();

    // First page (no cursor)
    let query = EdgeQuery::asc(&[b"item"]);
    let page1 = txn.find_edges(1, query).unwrap();
    assert_eq!(page1.len(), 10);

    // Second page using last item from page1 as cursor
    let last_edge = page1.last().unwrap();
    let cursor = EdgeCursor::new(&last_edge.sort_key, last_edge.dest);
    let query = EdgeQuery::asc(&[b"item"]).with_cursor(cursor);
    let page2 = txn.find_edges(1, query).unwrap();

    // Should be empty since we already got all items
    assert_eq!(page2.len(), 0);
}

#[test]
fn test_find_edges_pagination_desc() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    // Insert many edges
    let mut edges = Vec::new();
    for i in 1..=10 {
        edges.push((1, b"item" as &[u8], i * 10));
    }
    insert_edges(&txn, &edges).unwrap();

    // First page descending (no cursor)
    let query = EdgeQuery::desc(&[b"item"]);
    let page1 = txn.find_edges(1, query).unwrap();
    assert_eq!(page1.len(), 10);
    assert_eq!(page1[0].dest, 100); // Highest first

    // Second page using last item from page1 as cursor
    let last_edge = page1.last().unwrap();
    let cursor = EdgeCursor::new(&last_edge.sort_key, last_edge.dest);
    let query = EdgeQuery::desc(&[b"item"]).with_cursor(cursor);
    let page2 = txn.find_edges(1, query).unwrap();

    // Should be empty
    assert_eq!(page2.len(), 0);
}

#[test]
fn test_find_edges_limit_100() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    // Insert more than 100 edges
    let mut edges = Vec::new();
    for i in 1..=150 {
        edges.push((1, b"item" as &[u8], i));
    }
    insert_edges(&txn, &edges).unwrap();

    // Should only return 100
    let query = EdgeQuery::asc(&[b"item"]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 100);
    assert_eq!(result[0].dest, 1);
    assert_eq!(result[99].dest, 100);
}

#[test]
fn test_find_edges_cursor_boundary() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[(1, b"a", 1), (1, b"a", 2), (1, b"b", 1), (1, b"b", 2)],
    )
    .unwrap();

    // Cursor at edge type boundary ("a", 2)
    let cursor = EdgeCursor::new(b"a", 2);
    let query = EdgeQuery::asc(&[]).with_cursor(cursor);
    let result = txn.find_edges(1, query).unwrap();

    // Should get edges from type "b" only
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Edge::new(1, b"b".to_vec(), 1));
    assert_eq!(result[1], Edge::new(1, b"b".to_vec(), 2));
}

#[test]
fn test_find_edges_multiple_sources() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    insert_edges(
        &txn,
        &[
            (1, b"follows", 10),
            (1, b"follows", 20),
            (2, b"follows", 30),
            (2, b"follows", 40),
        ],
    )
    .unwrap();

    // Query for source 1
    let query = EdgeQuery::asc(&[b"follows"]);
    let result1 = txn.find_edges(1, query).unwrap();
    assert_eq!(result1.len(), 2);
    assert_eq!(result1[0].dest, 10);
    assert_eq!(result1[1].dest, 20);

    // Query for source 2
    let query = EdgeQuery::asc(&[b"follows"]);
    let result2 = txn.find_edges(2, query).unwrap();
    assert_eq!(result2.len(), 2);
    assert_eq!(result2[0].dest, 30);
    assert_eq!(result2[1].dest, 40);
}

#[test]
fn test_find_edges_binary_edge_types() {
    let conn = setup_db();
    let tx = conn.unchecked_transaction().unwrap();
    let txn = Txn::new(tx);

    // Use binary data as edge types
    insert_edges(
        &txn,
        &[
            (1, &[0x00, 0x01, 0x02], 10),
            (1, &[0x00, 0x01, 0x03], 20),
            (1, &[0xFF, 0xFE, 0xFD], 30),
        ],
    )
    .unwrap();

    // Query for specific binary edge type
    let query = EdgeQuery::asc(&[&[0x00, 0x01, 0x02]]);
    let result = txn.find_edges(1, query).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].sort_key, vec![0x00, 0x01, 0x02]);
    assert_eq!(result[0].dest, 10);
}
