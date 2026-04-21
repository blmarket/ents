use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use ents_postgres::PostgresDb;
use ents_test_suite::{run_all_tests, run_audit_tests};
use postgres::{Config, NoTls};
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

struct TestDb {
    db: PostgresDb,
    admin_config: Config,
    schema: String,
}

impl TestDb {
    fn setup() -> Result<Option<Self>> {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(value) => value,
            Err(_) => {
                eprintln!("skipping postgres tests: DATABASE_URL is unset");
                return Ok(None);
            }
        };

        let admin_config = database_url.parse::<Config>()?;
        let mut admin_client = admin_config.connect(NoTls)?;
        let schema = format!(
            "ents_postgres_test_{}_{}",
            process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        );
        admin_client.batch_execute(&format!("CREATE SCHEMA {schema}"))?;

        let mut pool_config = database_url.parse::<Config>()?;
        pool_config.options(&format!("-c search_path={schema}"));

        let manager = PostgresConnectionManager::new(pool_config, NoTls);
        let pool = Pool::builder().max_size(1).build(manager)?;
        let db = PostgresDb::from(pool);
        db.init()?;

        Ok(Some(Self {
            db,
            admin_config,
            schema,
        }))
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        if let Ok(mut client) = self.admin_config.connect(NoTls) {
            let _ = client.batch_execute(&format!(
                "DROP SCHEMA IF EXISTS {} CASCADE",
                self.schema
            ));
        }
    }
}

fn with_test_db<F>(f: F) -> Result<()>
where
    F: FnOnce(PostgresDb) -> Result<()>,
{
    let Some(test_db) = TestDb::setup()? else {
        return Ok(());
    };

    f(test_db.db.clone())
}

#[test]
fn test_all_postgres() -> Result<()> {
    with_test_db(run_all_tests)
}

#[test]
fn test_audit_postgres() -> Result<()> {
    with_test_db(run_audit_tests)
}
