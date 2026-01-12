use ents_admin_api::{admin_router, sqlite::SqlitePool};
use ents_test_suite::{Post, Tag, TestEntity, User};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let db_path = args
        .get(1)
        .expect("Please provide database file path as first argument");

    // Create SQLite database with shared pool
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;

    // Create admin backend from the same pool
    let mut backend = SqlitePool::from_pool(pool);

    // Register entity types for audit/fix operations
    backend.register_type::<TestEntity>();
    backend.register_type::<User>();
    backend.register_type::<Post>();
    backend.register_type::<Tag>();

    // Create the admin router
    let app = admin_router(backend);

    // Start the server
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Admin server running on http://localhost:8080");

    axum::serve(listener, app).await?;

    Ok(())
}
