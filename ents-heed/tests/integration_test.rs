use anyhow::Result;
use ents_heed::HeedEnv;
use ents_test_suite::run_all_tests;
use tempfile::TempDir;

#[test]
fn test_all_heed() -> Result<()> {
    // Create a temporary directory for the test database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    let env = HeedEnv::open(db_path, None)?;

    run_all_tests(env)?;

    Ok(())
}
