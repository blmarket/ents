use anyhow::Result;
use ents_sqlite::SqliteDb;
use ents_test_suite::{run_all_tests, run_audit_tests};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn setup_test_db() -> SqliteDb {
    let pool = Pool::new(SqliteConnectionManager::memory()).unwrap();
    let conn = pool.get().unwrap();
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS entities (
   id INTEGER PRIMARY KEY,
   type TEXT NOT NULL,
   data TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS edges (
   source INTEGER NOT NULL,
   type TEXT NOT NULL,
   dest INTEGER NOT NULL,
   PRIMARY KEY (source, type, dest)
);
"#,
    )
    .unwrap();
    SqliteDb::from(pool)
}

#[test]
fn test_all_sqlite() -> Result<()> {
    let db = setup_test_db();

    run_all_tests(db)?;

    Ok(())
}

#[test]
fn test_audit_sqlite() -> Result<()> {
    let db = setup_test_db();

    run_audit_tests(db)?;

    Ok(())
}
