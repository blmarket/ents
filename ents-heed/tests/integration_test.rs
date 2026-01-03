use anyhow::Result;
use ents_heed::{HeedEnv, Txn};
use ents_test_suite::{run_all_tests, TestCaseRunner, TestSuiteRunner};
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Clone)]
struct HeedTestRunner {
    env: Arc<HeedEnv>,
}

struct HeedCaseRunner {
    env: Arc<HeedEnv>,
}

impl TestCaseRunner for HeedCaseRunner {
    type Tx = Txn<'static>;

    fn execute<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Self::Tx) -> Result<R>,
    {
        let txn = self.env.write_txn()?;
        // Since the txn is consumed immediately in the closure, and the closure
        // executes synchronously, the env will still be alive during txn's use.
        let txn_static =
            unsafe { std::mem::transmute::<Txn<'_>, Txn<'static>>(txn) };
        f(txn_static)
    }
}

impl TestSuiteRunner for HeedTestRunner {
    type CaseRunner = HeedCaseRunner;

    fn create(&self) -> Result<Self::CaseRunner> {
        Ok(HeedCaseRunner {
            env: Arc::clone(&self.env),
        })
    }
}

#[test]
fn test_all_heed() -> Result<()> {
    // Create a temporary directory for the test database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let env = Arc::new(HeedEnv::open(db_path, None)?);
    let runner = HeedTestRunner { env };

    run_all_tests(runner)?;

    Ok(())
}
