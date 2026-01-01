use ents::{
    DraftError, EdgeDraft, EdgeProvider, EdgeQuery, QueryEdge, EdgeValue, Ent, EntExt as _,
    EntMutationError, EntWithEdges, Id, NullEdgeProvider, Transactional,
};
use ents_heed::HeedEnv;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

#[derive(Clone, Serialize, Deserialize)]
struct TestEntity {
    name: String,
    value: i32,
    id: Id,
    last_updated: u64,
}

#[typetag::serde]
impl Ent for TestEntity {
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn last_updated(&self) -> u64 {
        self.last_updated
    }
    fn mark_updated(&mut self) -> Result<(), EntMutationError> {
        self.last_updated = 12345; // Test value
        Ok(())
    }
}

impl EntWithEdges for TestEntity {
    type EdgeProvider = NullEdgeProvider;
}

impl TestEntity {
    pub fn build() -> TestEntityBuilder {
        TestEntityBuilder::default()
    }
}

#[derive(Default)]
struct TestEntityBuilder {
    name: String,
    value: i32,
    id: Id,
    last_updated: u64,
}

impl TestEntityBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
    pub fn value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }
    pub fn finish(self) -> anyhow::Result<TestEntity> {
        Ok(TestEntity {
            name: self.name,
            value: self.value,
            id: self.id,
            last_updated: self.last_updated,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct TestPerson {
    name: String,
    age: i32,
    lives_in_link: Id,
    id: Id,
    last_updated: u64,
}

#[typetag::serde]
impl Ent for TestPerson {
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn last_updated(&self) -> u64 {
        self.last_updated
    }
    fn mark_updated(&mut self) -> Result<(), EntMutationError> {
        self.last_updated = 12345;
        Ok(())
    }
}

impl TestPerson {
    pub fn lives_in_link(&self) -> &Id {
        &self.lives_in_link
    }
}

impl TestPerson {
    pub fn build() -> TestPersonBuilder {
        TestPersonBuilder::default()
    }
}

#[derive(Default)]
struct TestPersonBuilder {
    name: String,
    age: i32,
    lives_in_link: Id,
    id: Id,
    last_updated: u64,
}

impl TestPersonBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
    pub fn age(mut self, age: i32) -> Self {
        self.age = age;
        self
    }
    pub fn lives_in_link(mut self, lives_in_link: Id) -> Self {
        self.lives_in_link = lives_in_link;
        self
    }
    pub fn last_updated(mut self, last_updated: u64) -> Self {
        self.last_updated = last_updated;
        self
    }
    pub fn finish(self) -> anyhow::Result<TestPerson> {
        Ok(TestPerson {
            name: self.name,
            age: self.age,
            lives_in_link: self.lives_in_link,
            id: self.id,
            last_updated: self.last_updated,
        })
    }
}

#[derive(PartialEq)]
struct TestPersonEdgeDraft {
    person_id: Id,
    city_id: Id,
}

impl EdgeDraft for TestPersonEdgeDraft {
    fn check<T: Transactional>(self, _txn: &T) -> Result<Vec<EdgeValue>, DraftError> {
        Ok(vec![EdgeValue::new(
            self.person_id,
            b"lives_in".to_vec(),
            self.city_id,
        )])
    }
}

struct TestPersonEdgeProvider;
impl EdgeProvider<TestPerson> for TestPersonEdgeProvider {
    type Draft = TestPersonEdgeDraft;
    fn draft(ent: &TestPerson) -> Self::Draft {
        TestPersonEdgeDraft {
            person_id: ent.id(),
            city_id: *ent.lives_in_link(),
        }
    }
}

impl EntWithEdges for TestPerson {
    type EdgeProvider = TestPersonEdgeProvider;
}

impl TestPerson {
    pub fn set_lives_in_link(&mut self, lives_in_link: Id) {
        self.lives_in_link = lives_in_link;
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct TestCity {
    name: String,
    population: i64,
    id: Id,
    last_updated: u64,
}

#[typetag::serde]
impl Ent for TestCity {
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn last_updated(&self) -> u64 {
        self.last_updated
    }
    fn mark_updated(&mut self) -> Result<(), EntMutationError> {
        self.last_updated = 12345;
        Ok(())
    }
}

impl EntWithEdges for TestCity {
    type EdgeProvider = NullEdgeProvider;
}

impl TestCity {
    pub fn build() -> TestCityBuilder {
        TestCityBuilder::default()
    }
}

#[derive(Default)]
struct TestCityBuilder {
    name: String,
    population: i64,
}

impl TestCityBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
    pub fn population(mut self, population: i64) -> Self {
        self.population = population;
        self
    }
    pub fn finish(self) -> anyhow::Result<TestCity> {
        Ok(TestCity {
            name: self.name,
            population: self.population,
            id: 0,
            last_updated: 0,
        })
    }
}

fn setup_test_env() -> (tempfile::TempDir, HeedEnv) {
    let dir = tempdir().unwrap();
    let env = HeedEnv::open(dir.path(), None).unwrap();
    (dir, env)
}

#[test]
fn test_insert_and_get() {
    let (_dir, env) = setup_test_env();
    let txn = env.write_txn().unwrap();

    // Create an entity
    let ent = TestEntity::build()
        .name("test".to_string())
        .value(42)
        .finish()
        .unwrap();
    let id = txn.create(ent).unwrap();

    // Get the entity back
    let retrieved = txn.get(id).unwrap();
    assert!(retrieved.is_some());

    let retrieved_ent = retrieved.unwrap();
    assert_eq!(retrieved_ent.id(), id);
    assert!(retrieved_ent.is::<TestEntity>());
    assert_eq!(retrieved_ent.typetag_name(), "TestEntity");

    txn.commit().unwrap();
}

#[test]
fn test_get_nonexistent() {
    let (_dir, env) = setup_test_env();
    let txn = env.write_txn().unwrap();

    // Try to get a non-existent entity
    let result = txn.get(999).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_transaction_commit() {
    let (_dir, env) = setup_test_env();

    let id = {
        let txn = env.write_txn().unwrap();

        // Create an entity
        let ent = TestEntity::build()
            .name("committed".to_string())
            .value(999)
            .finish()
            .unwrap();
        let id = txn.create(ent).unwrap();

        // Commit the transaction
        txn.commit().unwrap();
        id
    };

    // Verify the entity persists after transaction commit
    let txn = env.write_txn().unwrap();
    let retrieved = txn.get(id).unwrap();
    assert!(retrieved.is_some());
}

#[test]
fn test_transaction_rollback() {
    let (_dir, env) = setup_test_env();

    let id = {
        let txn = env.write_txn().unwrap();

        // Create an entity
        let ent = TestEntity::build()
            .name("rolled_back".to_string())
            .value(888)
            .finish()
            .unwrap();
        let id = txn.create(ent).unwrap();

        // Transaction is dropped without commit, so it rolls back
        id
    };

    // Verify the entity does NOT persist after rollback
    let txn = env.write_txn().unwrap();
    let retrieved = txn.get(id).unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_update_without_cas() {
    let (_dir, env) = setup_test_env();
    let txn = env.write_txn().unwrap();

    // Create an entity
    let mut ent = TestEntity::build()
        .name("original".to_string())
        .value(100)
        .finish()
        .unwrap();
    let id = txn.create(ent.clone()).unwrap();
    ent.set_id(id);

    // Update using update
    let success = txn
        .update(&mut ent, |e: &mut TestEntity| {
            e.name = "updated".to_string();
            e.value = 200;
        })
        .unwrap();
    assert!(success);

    // Verify the update
    let retrieved = txn.get(id).unwrap().unwrap();
    let retrieved_json = serde_json::to_value(&retrieved).unwrap();
    assert_eq!(retrieved_json["name"], "updated");
    assert_eq!(retrieved_json["value"], 200);
}

#[test]
fn test_update_edge_change() {
    let (_dir, env) = setup_test_env();
    let txn = env.write_txn().unwrap();

    // Create cities
    let city1 = TestCity::build()
        .name("City1".to_string())
        .population(100)
        .finish()
        .unwrap();
    let city1_id = txn.create(city1).unwrap();

    let city2 = TestCity::build()
        .name("City2".to_string())
        .population(200)
        .finish()
        .unwrap();
    let city2_id = txn.create(city2).unwrap();

    // Create person living in city1
    let mut person = TestPerson::build()
        .name("Alice".to_string())
        .age(30)
        .lives_in_link(city1_id)
        .last_updated(0)
        .finish()
        .unwrap();
    let person_id = txn.create(person.clone()).unwrap();
    person.set_id(person_id);

    // Verify edge to city1
    let edges = txn.find_edges(person_id, EdgeQuery::asc(&[])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].dest, city1_id);

    // Update person to live in city2
    let success = txn
        .update(&mut person, |p: &mut TestPerson| {
            p.set_lives_in_link(city2_id);
        })
        .unwrap();
    assert!(success);

    // Verify edge changed to city2
    let edges = txn.find_edges(person_id, EdgeQuery::asc(&[])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].dest, city2_id);

    person.set_lives_in_link(city2_id);

    // Update person to live in city2 (no change)
    // This should trigger the optimization path
    let success_no_change = txn
        .update(&mut person, |p: &mut TestPerson| {
            p.set_lives_in_link(city2_id);
        })
        .unwrap();
    assert!(success_no_change);

    // Verify edge is still city2
    let edges = txn.find_edges(person_id, EdgeQuery::asc(&[])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].dest, city2_id);
}
