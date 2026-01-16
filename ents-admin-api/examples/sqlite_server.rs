use ents_admin_api::{admin_router, sqlite::SqlitePool, TypeRegistryBuilder};
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

    // Register entity types for audit/fix operations
    // The type parameter specifies the transaction type for this backend
    let registry = TypeRegistryBuilder::<ents_sqlite::Txn<'static>>::new()
        .register::<TestEntity>()
        .register::<User>()
        .register::<Post>()
        .register::<Tag>()
        .build();

    // Create admin backend from the same pool with registry
    let backend = SqlitePool::from_pool(pool).with_registry(registry);

    // Create the admin router
    let app = admin_router(backend);

    // Start the server
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Admin server running on http://localhost:8080");

    axum::serve(listener, app).await?;

    Ok(())
}
