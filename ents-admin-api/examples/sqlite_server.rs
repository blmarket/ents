use ents_admin_api::{admin_router, AdminBackendProvider, TypeRegistryBuilder};
use ents_sqlite::SqliteDb;
use ents_test_suite::{Post, Tag, TestEntity, User};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SQLite database with shared pool
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::new(manager)?;
    pool.get()?.execute_batch(
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
        CREATE INDEX IF NOT EXISTS edges_dest ON edges(dest);"#,
    )?;

    // Register entity types for audit/fix operations
    // The type parameter specifies the transaction type for this backend
    let registry = TypeRegistryBuilder::<ents_sqlite::Txn<'static>>::new()
        .register::<TestEntity>()
        .register::<User>()
        .register::<Post>()
        .register::<Tag>()
        .build();

    let backend = AdminBackendProvider::new(SqliteDb::new(pool), registry);

    // Create the admin router
    let app = admin_router(backend);

    // Optionally serve static assets if ASSET_PATH is set
    // For SPA routing, serve index.html for any path that doesn't match a static file
    let app = if let Ok(asset_path) = std::env::var("ASSET_PATH") {
        println!("Serving static assets from: {}", asset_path);
        let index_path = format!("{}/index.html", asset_path);
        let serve_dir =
            ServeDir::new(&asset_path).fallback(ServeFile::new(&index_path));
        app.fallback_service(serve_dir)
    } else {
        app
    };

    // Start the server
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Admin server running on http://localhost:8080");

    axum::serve(listener, app).await?;

    Ok(())
}
