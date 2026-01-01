//! Basic CRUD operations example
//!
//! This example demonstrates creating, reading, updating, and deleting entities
//! with persistent storage in LMDB.
//!
//! Run with: cargo run --example basic_crud

use ents::{Ent, EntMutationError, EntWithEdges, Id, NullEdgeProvider, Transactional};
use ents_heed::HeedEnv;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
    age: u32,
    id: Id,
    last_updated: u64,
}

#[typetag::serde]
impl Ent for User {
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
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }
}

impl EntWithEdges for User {
    type EdgeProvider = NullEdgeProvider;
}

impl User {
    fn new(name: String, email: String, age: u32) -> Self {
        Self {
            name,
            email,
            age,
            id: 0,
            last_updated: 0,
        }
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== Basic CRUD Example ===\n");

    // Open database with persistent storage
    let env = HeedEnv::open("./var/basic_crud", None)?;

    // CREATE: Insert a new user
    println!("Creating user...");
    let mut user = User::new(
        "Alice Johnson".to_string(),
        "alice@example.com".to_string(),
        28,
    );
    let user_id = {
        let txn = env.write_txn()?;
        let id = txn.create(user.clone())?;
        txn.commit()?;
        user.set_id(id);
        println!("✓ Created user with ID: {}", id);
        id
    };

    // READ: Retrieve the user
    println!("\nReading user...");
    {
        let txn = env.write_txn()?;
        if let Some(user_ent) = txn.get(user_id)? {
            let user_json = serde_json::to_value(&user_ent)?;
            println!(
                "✓ Retrieved user: {}",
                serde_json::to_string_pretty(&user_json)?
            );
        }
    }

    // UPDATE: Modify the user
    println!("\nUpdating user...");
    {
        let txn = env.write_txn()?;
        let updated = txn.update(&mut user, |u: &mut User| {
            u.age = 29;
            u.email = "alice.j@example.com".to_string();
        })?;

        if updated {
            txn.commit()?;
            println!("✓ User updated successfully");

            // Show updated data
            let txn = env.write_txn()?;
            if let Some(user_ent) = txn.get(user_id)? {
                let user_json = serde_json::to_value(&user_ent)?;
                println!("  New data: {}", serde_json::to_string_pretty(&user_json)?);
            }
        }
    }

    // DELETE: Remove the user
    println!("\nDeleting user...");
    {
        let txn = env.write_txn()?;
        txn.delete::<User>(user_id)?;
        txn.commit()?;
        println!("✓ User deleted");
    }

    // VERIFY deletion
    println!("\nVerifying deletion...");
    {
        let txn = env.write_txn()?;
        match txn.get(user_id)? {
            Some(_) => println!("✗ User still exists!"),
            None => println!("✓ User successfully removed"),
        }
    }

    println!("\n=== Example Complete ===");
    println!("Data persisted to: ./var/basic_crud");

    Ok(())
}
