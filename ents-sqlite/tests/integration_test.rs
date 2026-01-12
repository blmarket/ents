use anyhow::Result;
use ents_sqlite::Txn;
use ents_test_suite::{
    run_all_tests, run_audit_tests, AdminTestCaseRunner, AdminTestSuiteRunner,
    TestCaseRunner, TestSuiteRunner,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

#[derive(Clone)]
struct SqliteTestRunner {
    pool: Pool<SqliteConnectionManager>,
}

struct SqliteCaseRunner {
    pool: Pool<SqliteConnectionManager>,
}

impl TestCaseRunner for SqliteCaseRunner {
    type Tx = Txn<'static>;

    fn execute<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Self::Tx) -> Result<R>,
    {
        let mut conn = self.pool.get().map_err(anyhow::Error::from)?;
        let tx = conn.transaction().map_err(anyhow::Error::from)?;
        let txn = Txn::new(tx);
        // Since the txn is consumed immediately in the closure, and the closure
        // executes synchronously, the conn will still be alive during txn's use.
        let txn_static =
            unsafe { std::mem::transmute::<Txn<'_>, Txn<'static>>(txn) };
        f(txn_static)
    }
}

impl TestSuiteRunner for SqliteTestRunner {
    type CaseRunner = SqliteCaseRunner;

    fn create(&self) -> Result<Self::CaseRunner> {
        Ok(SqliteCaseRunner {
            pool: self.pool.clone(),
        })
    }
}

// Implement AuditTestCaseRunner for SqliteCaseRunner
// (Sqlite's Txn implements AdminEdgeByDest, so it supports audit tests)
impl AdminTestCaseRunner for SqliteCaseRunner {
    type Tx = Txn<'static>;

    fn execute<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Self::Tx) -> Result<R>,
    {
        let mut conn = self.pool.get().map_err(anyhow::Error::from)?;
        let tx = conn.transaction().map_err(anyhow::Error::from)?;
        let txn = Txn::new(tx);
        let txn_static =
            unsafe { std::mem::transmute::<Txn<'_>, Txn<'static>>(txn) };
        f(txn_static)
    }
}

impl AdminTestSuiteRunner for SqliteTestRunner {
    type CaseRunner = SqliteCaseRunner;

    fn create(&self) -> Result<Self::CaseRunner> {
        Ok(SqliteCaseRunner {
            pool: self.pool.clone(),
        })
    }
}

fn setup_test_db() -> Pool<SqliteConnectionManager> {
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
    pool
}

#[test]
fn test_all_sqlite() -> Result<()> {
    let pool = setup_test_db();
    let runner = SqliteTestRunner { pool };

    run_all_tests(runner)?;

    Ok(())
}

#[test]
fn test_audit_sqlite() -> Result<()> {
    let pool = setup_test_db();
    let runner = SqliteTestRunner { pool };

    run_audit_tests(runner)?;

    Ok(())
}
