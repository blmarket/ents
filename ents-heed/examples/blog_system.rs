//! Blog system with posts and authors
//!
//! This example demonstrates a more complex entity system with relationships
//! between blog posts and authors using edges.
//!
//! Run with: cargo run --example blog_system

use ents::{
    DraftError, EdgeDraft, EdgeProvider, EdgeQuery, EdgeValue, Ent, EntExt,
    EntMutationError, EntWithEdges, Id, NullEdgeProvider, QueryEdge,
    Transactional,
};
use ents_heed::HeedEnv;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct Author {
    name: String,
    bio: String,
    id: Id,
    last_updated: u64,
}

#[typetag::serde]
impl Ent for Author {
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

impl EntWithEdges for Author {
    type EdgeProvider = NullEdgeProvider;
}

impl Author {
    fn new(name: String, bio: String) -> Self {
        Self {
            name,
            bio,
            id: 0,
            last_updated: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct BlogPost {
    title: String,
    content: String,
    author_id: Id,
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

#[derive(PartialEq)]
struct BlogPostEdgeDraft {
    post_id: Id,
    author_id: Id,
}

impl EdgeDraft for BlogPostEdgeDraft {
    fn check<T: Transactional>(
        self,
        _txn: &T,
    ) -> Result<Vec<EdgeValue>, DraftError> {
        Ok(vec![EdgeValue::new(
            self.post_id,
            b"authored_by".to_vec(),
            self.author_id,
        )])
    }
}

struct BlogPostEdgeProvider;
impl EdgeProvider<BlogPost> for BlogPostEdgeProvider {
    type Draft = BlogPostEdgeDraft;
    fn draft(ent: &BlogPost) -> Self::Draft {
        BlogPostEdgeDraft {
            post_id: ent.id(),
            author_id: ent.author_id,
        }
    }
}

impl EntWithEdges for BlogPost {
    type EdgeProvider = BlogPostEdgeProvider;
}

impl BlogPost {
    fn new(title: String, content: String, author_id: Id) -> Self {
        Self {
            title,
            content,
            author_id,
            id: 0,
            last_updated: 0,
        }
    }

    fn set_author(&mut self, author_id: Id) {
        self.author_id = author_id;
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== Blog System Example ===\n");

    let env = HeedEnv::open("./var/blog_system", None)?;

    // Create authors
    println!("Creating authors...");
    let alice_id = {
        let txn = env.write_txn()?;
        let author = Author::new(
            "Alice Smith".to_string(),
            "Tech enthusiast and developer".to_string(),
        );
        let id = txn.create(author)?;
        txn.commit()?;
        println!("✓ Created author 'Alice Smith' (ID: {})", id);
        id
    };

    let bob_id = {
        let txn = env.write_txn()?;
        let author = Author::new(
            "Bob Jones".to_string(),
            "Science writer and researcher".to_string(),
        );
        let id = txn.create(author)?;
        txn.commit()?;
        println!("✓ Created author 'Bob Jones' (ID: {})", id);
        id
    };

    // Create blog posts
    println!("\nCreating blog posts...");
    let mut post1 = BlogPost::new(
        "Getting Started with Rust".to_string(),
        "Rust is a systems programming language...".to_string(),
        alice_id,
    );
    let post1_id = {
        let txn = env.write_txn()?;
        let id = txn.create(post1.clone())?;
        txn.commit()?;
        post1.set_id(id);
        println!("✓ Created post 'Getting Started with Rust' (ID: {})", id);
        id
    };

    let mut post2 = BlogPost::new(
        "Advanced Rust Patterns".to_string(),
        "Let's explore some advanced patterns...".to_string(),
        alice_id,
    );
    let post2_id = {
        let txn = env.write_txn()?;
        let id = txn.create(post2.clone())?;
        txn.commit()?;
        post2.set_id(id);
        println!("✓ Created post 'Advanced Rust Patterns' (ID: {})", id);
        id
    };

    let post3_id = {
        let txn = env.write_txn()?;
        let post = BlogPost::new(
            "The Science of Computing".to_string(),
            "Understanding the fundamentals...".to_string(),
            bob_id,
        );
        let id = txn.create(post)?;
        txn.commit()?;
        println!("✓ Created post 'The Science of Computing' (ID: {})", id);
        id
    };

    // Query edges to find posts by author
    println!("\nFinding posts by Alice...");
    {
        let txn = env.write_txn()?;

        // Find all edges from posts to alice
        for post_id in [post1_id, post2_id, post3_id] {
            let edges =
                txn.find_edges(post_id, EdgeQuery::asc(&[b"authored_by"]))?;

            if !edges.is_empty() && edges[0].dest == alice_id {
                if let Some(post_ent) = txn.get(post_id)? {
                    if post_ent.is::<BlogPost>() {
                        let post_json = serde_json::to_value(&post_ent)?;
                        println!(
                            "  ✓ Found: {}",
                            post_json["title"].as_str().unwrap()
                        );
                    }
                }
            }
        }
    }

    // Update a post's author
    println!("\nTransferring 'Advanced Rust Patterns' to Bob...");
    {
        let txn = env.write_txn()?;
        let updated = txn.update(&mut post2, |p: &mut BlogPost| {
            p.set_author(bob_id);
        })?;

        if updated {
            txn.commit()?;
            println!("✓ Post transferred");

            // Verify edge changed
            let txn = env.write_txn()?;
            let edges =
                txn.find_edges(post2_id, EdgeQuery::asc(&[b"authored_by"]))?;
            if !edges.is_empty() && edges[0].dest == bob_id {
                println!("✓ Edge updated correctly");
            }
        }
    }

    // Count posts per author
    println!("\nPost count per author:");
    {
        let txn = env.write_txn()?;

        let mut alice_count = 0;
        let mut bob_count = 0;

        for post_id in [post1_id, post2_id, post3_id] {
            let edges =
                txn.find_edges(post_id, EdgeQuery::asc(&[b"authored_by"]))?;
            if !edges.is_empty() {
                if edges[0].dest == alice_id {
                    alice_count += 1;
                } else if edges[0].dest == bob_id {
                    bob_count += 1;
                }
            }
        }

        println!("  Alice: {} post(s)", alice_count);
        println!("  Bob: {} post(s)", bob_count);
    }

    println!("\n=== Example Complete ===");
    println!("Data persisted to: ./var/blog_system");

    Ok(())
}
