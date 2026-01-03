//! Simple blog example
//!
//! A straightforward example showing multiple blog posts without complex edges.
//!
//! Run with: cargo run --example simple_blog

use ents::{
    Ent, EntMutationError, EntWithEdges, Id, NullEdgeProvider, Transactional,
};
use ents_heed::HeedEnv;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct BlogPost {
    title: String,
    content: String,
    author: String,
    views: u64,
    id: Id,
    last_updated: u64,
}

#[typetag::serde]
impl Ent for BlogPost {
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
            .as_micros() as u64;
        Ok(())
    }
}

impl EntWithEdges for BlogPost {
    type EdgeProvider = NullEdgeProvider;
}

impl BlogPost {
    fn new(title: String, author: String, content: String) -> Self {
        Self {
            title,
            content,
            author,
            views: 0,
            id: 0,
            last_updated: 0,
        }
    }

    fn increment_views(&mut self) {
        self.views += 1;
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== Simple Blog Example ===\n");

    let env = HeedEnv::open("./var/simple_blog", None)?;

    // Create some blog posts
    println!("Creating blog posts...");

    let mut post1 = BlogPost::new(
        "Introduction to LMDB".to_string(),
        "Alice".to_string(),
        "LMDB is a fast key-value database...".to_string(),
    );
    let post1_id = {
        let txn = env.write_txn()?;
        let id = txn.create(post1.clone())?;
        txn.commit()?;
        post1.set_id(id);
        println!("✓ Created: '{}'", post1.title);
        id
    };

    let mut post2 = BlogPost::new(
        "Rust Performance Tips".to_string(),
        "Bob".to_string(),
        "Here are some ways to optimize your Rust code...".to_string(),
    );
    let post2_id = {
        let txn = env.write_txn()?;
        let id = txn.create(post2.clone())?;
        txn.commit()?;
        post2.set_id(id);
        println!("✓ Created: '{}'", post2.title);
        id
    };

    // Simulate viewing posts
    println!("\nSimulating page views...");
    for _ in 0..5 {
        let txn = env.write_txn()?;
        txn.update(&mut post1, |p: &mut BlogPost| {
            p.increment_views();
        })?;
        txn.commit()?;
    }
    println!("✓ Post 1 viewed 5 times");

    for _ in 0..3 {
        let txn = env.write_txn()?;
        txn.update(&mut post2, |p: &mut BlogPost| {
            p.increment_views();
        })?;
        txn.commit()?;
    }
    println!("✓ Post 2 viewed 3 times");

    // Display post statistics
    println!("\nPost Statistics:");
    {
        let txn = env.write_txn()?;

        for post_id in [post1_id, post2_id] {
            if let Some(post_ent) = txn.get(post_id)? {
                let json = serde_json::to_value(&post_ent)?;
                println!(
                    "  '{}' by {} - {} views",
                    json["title"].as_str().unwrap(),
                    json["author"].as_str().unwrap(),
                    json["views"].as_u64().unwrap()
                );
            }
        }
    }

    println!("\n=== Example Complete ===");
    println!("Data persisted to: ./var/simple_blog");

    Ok(())
}
