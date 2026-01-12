//! Demo: Set up a SQLite database with example entities and edges.
//!
//! This example creates a `database.db` file with the necessary tables
//! and populates it with sample Users, Tags, and Posts entities along
//! with their relationship edges.
//!
//! Run with: cargo run --example setup-database

use ents::Transactional;
use ents_sqlite::Txn;
use ents_test_suite::{Post, Tag, User};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "database.db";

    // Remove existing database if present
    if std::path::Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
        println!("Removed existing database file");
    }

    // Create SQLite connection pool
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;
    let conn = pool.get()?;

    // Create schema
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            data TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS edges (
            source INTEGER NOT NULL,
            type BLOB NOT NULL,
            dest INTEGER NOT NULL,
            PRIMARY KEY (source, type, dest)
        );
        CREATE INDEX IF NOT EXISTS edges_dest ON edges(dest);
        "#,
    )?;
    println!("Created database schema");

    // Drop the connection so we can use the pool
    drop(conn);

    // Create entities using a transaction
    let mut conn = pool.get()?;
    let txn = Txn::new(conn.transaction()?);

    // Create users
    let alice = User::new("alice".to_string(), "alice@example.com".to_string());
    let alice_id = txn.create(alice)?;
    println!("Created user Alice (id: {})", alice_id);

    let bob = User::new("bob".to_string(), "bob@example.com".to_string());
    let bob_id = txn.create(bob)?;
    println!("Created user Bob (id: {})", bob_id);

    let charlie =
        User::new("charlie".to_string(), "charlie@example.com".to_string());
    let charlie_id = txn.create(charlie)?;
    println!("Created user Charlie (id: {})", charlie_id);

    // Create tags
    let rust_tag = Tag::new("rust".to_string(), "#ff6b6b".to_string());
    let rust_tag_id = txn.create(rust_tag)?;
    println!("Created tag 'rust' (id: {})", rust_tag_id);

    let programming_tag =
        Tag::new("programming".to_string(), "#4ecdc4".to_string());
    let programming_tag_id = txn.create(programming_tag)?;
    println!("Created tag 'programming' (id: {})", programming_tag_id);

    let tutorial_tag = Tag::new("tutorial".to_string(), "#45b7d1".to_string());
    let tutorial_tag_id = txn.create(tutorial_tag)?;
    println!("Created tag 'tutorial' (id: {})", tutorial_tag_id);

    let webdev_tag = Tag::new("webdev".to_string(), "#f9ca24".to_string());
    let webdev_tag_id = txn.create(webdev_tag)?;
    println!("Created tag 'webdev' (id: {})", webdev_tag_id);

    let database_tag = Tag::new("database".to_string(), "#6c5ce7".to_string());
    let database_tag_id = txn.create(database_tag)?;
    println!("Created tag 'database' (id: {})", database_tag_id);

    // Create posts with relationships
    let post1 = Post::new(
        "Getting Started with Rust".to_string(),
        "Rust is a systems programming language that runs blazingly fast..."
            .to_string(),
        alice_id,
        vec![rust_tag_id, programming_tag_id, tutorial_tag_id],
    );
    let post1_id = txn.create(post1)?;
    println!(
        "Created post '{}' by Alice (id: {})",
        "Getting Started with Rust", post1_id
    );

    let post2 = Post::new(
        "Building Web APIs with Axum".to_string(),
        "Axum is a web framework that focuses on ergonomics and modularity..."
            .to_string(),
        bob_id,
        vec![rust_tag_id, webdev_tag_id],
    );
    let post2_id = txn.create(post2)?;
    println!(
        "Created post '{}' by Bob (id: {})",
        "Building Web APIs with Axum", post2_id
    );

    let post3 = Post::new(
        "SQLite for Embedded Applications".to_string(),
        "SQLite is a lightweight, serverless database perfect for embedded use...".to_string(),
        charlie_id,
        vec![database_tag_id, programming_tag_id],
    );
    let post3_id = txn.create(post3)?;
    println!(
        "Created post '{}' by Charlie (id: {})",
        "SQLite for Embedded Applications", post3_id
    );

    let post4 = Post::new(
        "Advanced Rust Patterns".to_string(),
        "Let's explore some advanced patterns in Rust including traits and generics...".to_string(),
        alice_id,
        vec![rust_tag_id, programming_tag_id],
    );
    let post4_id = txn.create(post4)?;
    println!(
        "Created post '{}' by Alice (id: {})",
        "Advanced Rust Patterns", post4_id
    );

    // Commit the transaction
    txn.commit()?;
    println!("\nAll entities and edges committed to database");

    // Print summary
    println!("\n=== Database Summary ===");
    println!("Users: {} (Alice, Bob, Charlie)", 3);
    println!(
        "Tags: {} (rust, programming, tutorial, webdev, database)",
        5
    );
    println!("Posts: {}", 4);
    println!("\nDatabase file created at: {}", db_path);
    println!("\nYou can now run the admin server with:");
    println!("  cargo run --example sqlite_server -- {}", db_path);

    Ok(())
}
