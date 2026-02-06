//! Example demonstrating how to enumerate edges from Box<dyn Ent> using TypeId-based dispatch.
//!
//! This example shows:
//! 1. How to create a type-safe EdgeProvider registry using TypeId
//! 2. How to enumerate edges from Box<dyn Ent> without knowing concrete types
//! 3. How to validate edges using transactions
//! 4. How to work with SQLite database
//! 5. How to enforce unique constraints using edges (UserNameIndex -> User)
//! 6. How to use AuditEntEdges trait for edge auditing

use ents::{
    DraftError, EdgeDraft, EdgeQuery, EdgeValue, Ent, EntExt, EntMutationError,
    check_incoming_edges, Id, IncomingEdgeProvider, IncomingEdgeValue,
    NullEdgeProvider, QueryEdge, ReadEnt, Transactional,
};
use ents_admin::AdminEnt;
use ents_sqlite::Txn;
use r2d2_sqlite::rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;

// ============================================================================
// Entity Definitions
// ============================================================================

/// A singleton entity that serves as the source for username uniqueness index edges.
/// The edge from UserNameIndex -> User uses the username as the sort key, ensuring uniqueness.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserNameIndex {
    pub id: Id,
    pub last_updated: u64,
}

#[typetag::serde]
impl Ent for UserNameIndex {
    type EdgeProvider = NullEdgeProvider;

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

impl UserNameIndex {
    pub fn new() -> Self {
        Self {
            id: 0,
            last_updated: 0,
        }
    }
}

/// A User entity with unique username constraint enforced via edge to UserNameIndex
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub email: String,
    /// The ID of the UserNameIndex singleton that owns the uniqueness edge
    pub username_index_id: Id,
    pub id: Id,
    pub last_updated: u64,
}

/// Edge draft for enforcing unique username constraint.
/// Creates an edge from UserNameIndex -> User with username as the sort key.
#[derive(PartialEq, Debug)]
pub struct UniqueUsernameDraft {
    pub user_id: Id,
    pub username_index_id: Id,
    pub username: String,
}

impl EdgeDraft for UniqueUsernameDraft {
    fn check<T: ReadEnt>(
        self,
        txn: &T,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError> {
        // Check if any existing edge has this username as the sort key
        let username_bytes = self.username.as_bytes();
        let existing_result = txn.find_edges(
            self.username_index_id,
            EdgeQuery::asc(&[username_bytes]),
        )?;

        // If we find an edge with exactly this username, the username is already taken.
        for edge in &existing_result.edges {
            if edge.sort_key == username_bytes {
                return Err(DraftError::ValidationFailed(format!(
                    "Username '{}' is already taken by user {}",
                    self.username, edge.dest
                )));
            }
        }

        // Create the unique username edge: UserNameIndex -> User with username as sort key
        Ok(vec![IncomingEdgeValue::new(
            self.username_index_id,
            username_bytes.to_vec(),
        )])
    }
}

/// Edge provider for User that creates unique username edge
pub struct UserEdgeProvider;

impl IncomingEdgeProvider<User> for UserEdgeProvider {
    type Draft = UniqueUsernameDraft;

    fn draft(ent: &User) -> Self::Draft {
        UniqueUsernameDraft {
            user_id: ent.id(),
            username_index_id: ent.username_index_id,
            username: ent.username.clone(),
        }
    }
}

#[typetag::serde]
impl Ent for User {
    type EdgeProvider = UserEdgeProvider;

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

impl User {
    pub fn new(username: String, email: String, username_index_id: Id) -> Self {
        Self {
            username,
            email,
            username_index_id,
            id: 0,
            last_updated: 0,
        }
    }
}

/// A Post entity with edges to User (author) and other Posts (tags)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Post {
    pub title: String,
    pub content: String,
    pub author_id: Id,
    pub tag_ids: Vec<Id>,
    pub id: Id,
    pub last_updated: u64,
}

/// Edge draft for author relationship (User -> Post)
/// The edge is from the author (User) to the Post, so Post is the destination.
/// This correctly implements IncomingEdgeProvider semantics where the entity is the dest.
#[derive(PartialEq, Debug)]
pub struct AuthorEdgeDraft {
    pub post_id: Id,
    pub author_id: Id,
}

impl EdgeDraft for AuthorEdgeDraft {
    fn check<T: ReadEnt>(
        self,
        txn: &T,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError> {
        // Validate that the author exists
        let author = txn.get(self.author_id)?;
        if author.is_none() {
            return Err(DraftError::SourceNotFound(self.author_id));
        }

        // Edge: User --[authored]--> Post
        // The Post is the destination (incoming edge to Post)
        Ok(vec![IncomingEdgeValue::new(
            self.author_id,
            b"authored".to_vec(),
        )])
    }
}

/// Edge draft for tag relationships (Tag -> Post)
/// The edge is from the tag source to the Post, so Post is the destination.
/// This correctly implements IncomingEdgeProvider semantics where the entity is the dest.
#[derive(PartialEq, Debug)]
pub struct TagsEdgeDraft {
    pub post_id: Id,
    pub tag_ids: Vec<Id>,
}

impl EdgeDraft for TagsEdgeDraft {
    fn check<T: ReadEnt>(
        self,
        txn: &T,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError> {
        let mut edges = Vec::new();

        // Validate that all tag sources exist
        for tag_id in &self.tag_ids {
            let tag = txn.get(*tag_id)?;
            if tag.is_none() {
                return Err(DraftError::SourceNotFound(*tag_id));
            }
            // Edge: Tag --[tagged]--> Post
            // The Post is the destination (incoming edge to Post)
            edges.push(IncomingEdgeValue::new(
                *tag_id,
                b"tagged".to_vec(),
            ));
        }

        Ok(edges)
    }
}

/// Edge provider for Post that creates both author and tag edges
pub struct PostEdgeProvider;

impl IncomingEdgeProvider<Post> for PostEdgeProvider {
    type Draft = (AuthorEdgeDraft, TagsEdgeDraft);

    fn draft(ent: &Post) -> Self::Draft {
        (
            AuthorEdgeDraft {
                post_id: ent.id,
                author_id: ent.author_id,
            },
            TagsEdgeDraft {
                post_id: ent.id,
                tag_ids: ent.tag_ids.clone(),
            },
        )
    }
}

#[typetag::serde]
impl Ent for Post {
    type EdgeProvider = PostEdgeProvider;

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

impl Post {
    pub fn new(
        title: String,
        content: String,
        author_id: Id,
        tag_ids: Vec<Id>,
    ) -> Self {
        Self {
            title,
            content,
            author_id,
            tag_ids,
            id: 0,
            last_updated: 0,
        }
    }
}

/// A Comment entity with edges to Post and User (author)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Comment {
    pub content: String,
    pub post_id: Id,
    pub author_id: Id,
    pub id: Id,
    pub last_updated: u64,
}

/// Edge draft for comment relationships (Post -> Comment, User -> Comment)
/// The edges are from Post and User to the Comment, so Comment is the destination.
/// This correctly implements IncomingEdgeProvider semantics where the entity is the dest.
#[derive(PartialEq, Debug)]
pub struct CommentEdgeDraft {
    pub comment_id: Id,
    pub post_id: Id,
    pub author_id: Id,
}

impl EdgeDraft for CommentEdgeDraft {
    fn check<T: ReadEnt>(
        self,
        txn: &T,
    ) -> Result<Vec<IncomingEdgeValue>, DraftError> {
        // Validate that the post exists (source of edge)
        let post = txn.get(self.post_id)?;
        if post.is_none() {
            return Err(DraftError::SourceNotFound(self.post_id));
        }

        // Validate that the author exists (source of edge)
        let author = txn.get(self.author_id)?;
        if author.is_none() {
            return Err(DraftError::SourceNotFound(self.author_id));
        }

        // Edge: Post --[has_comment]--> Comment
        // Edge: User --[commented]--> Comment
        // The Comment is the destination (incoming edges to Comment)
        Ok(vec![
            IncomingEdgeValue::new(self.post_id, b"has_comment".to_vec()),
            IncomingEdgeValue::new(self.author_id, b"commented".to_vec()),
        ])
    }
}

/// Edge provider for Comment
pub struct CommentEdgeProvider;

impl IncomingEdgeProvider<Comment> for CommentEdgeProvider {
    type Draft = CommentEdgeDraft;

    fn draft(ent: &Comment) -> Self::Draft {
        CommentEdgeDraft {
            comment_id: ent.id,
            post_id: ent.post_id,
            author_id: ent.author_id,
        }
    }
}

#[typetag::serde]
impl Ent for Comment {
    type EdgeProvider = CommentEdgeProvider;

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

impl Comment {
    pub fn new(content: String, post_id: Id, author_id: Id) -> Self {
        Self {
            content,
            post_id,
            author_id,
            id: 0,
            last_updated: 0,
        }
    }
}

// ============================================================================
// EdgeProvider Registry - The Key Part
// ============================================================================

/// A trait for dynamically enumerating edges from Box<dyn Ent>
trait EdgeEnumerator: Send + Sync {
    fn enumerate_edges(
        &self,
        ent: &Box<dyn Ent>,
        txn: &Txn,
    ) -> Result<Vec<EdgeValue>, DraftError>;
}

/// Generic implementation of EdgeEnumerator for any entity type with EdgeProvider
struct TypedEdgeEnumerator<E: Ent> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Ent> TypedEdgeEnumerator<E> {
    fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Ent> EdgeEnumerator for TypedEdgeEnumerator<E> {
    fn enumerate_edges(
        &self,
        ent: &Box<dyn Ent>,
        txn: &Txn,
    ) -> Result<Vec<EdgeValue>, DraftError> {
        // Try to downcast to concrete type
        if let Some(concrete_ent) = ent.as_ent::<E>() {
            // Use the EdgeProvider associated type to draft edges
            check_incoming_edges(concrete_ent, txn)
        } else {
            Err(DraftError::ValidationFailed(
                "Failed to downcast entity".to_string(),
            ))
        }
    }
}

/// Registry that maps TypeId to EdgeEnumerator
pub struct EdgeProviderRegistry {
    enumerators: HashMap<TypeId, Box<dyn EdgeEnumerator>>,
}

impl EdgeProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            enumerators: HashMap::new(),
        }
    }

    /// Register an entity type with its EdgeProvider
    pub fn register<E: Ent>(&mut self) {
        let type_id = TypeId::of::<E>();
        let enumerator = Box::new(TypedEdgeEnumerator::<E>::new());
        self.enumerators.insert(type_id, enumerator);
    }

    /// Enumerate edges from a Box<dyn Ent> using the registered EdgeProvider
    pub fn enumerate_edges(
        &self,
        ent: &Box<dyn Ent>,
        txn: &Txn,
    ) -> Result<Vec<EdgeValue>, DraftError> {
        // Get TypeId from the dyn Ent
        let type_id = (&**ent as &dyn Any).type_id();

        // Look up the enumerator
        if let Some(enumerator) = self.enumerators.get(&type_id) {
            enumerator.enumerate_edges(ent, txn)
        } else {
            Err(DraftError::ValidationFailed(format!(
                "No EdgeProvider registered for type {:?}",
                type_id
            )))
        }
    }
}

// ============================================================================
// Database Setup
// ============================================================================

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute(
        "CREATE TABLE entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            data TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE edges (
            source INTEGER NOT NULL,
            type BLOB NOT NULL,
            dest INTEGER NOT NULL,
            PRIMARY KEY (source, type, dest)
        )",
        [],
    )
    .unwrap();

    conn
}

// ============================================================================
// Main Example
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Edge Enumeration Example ===\n");

    // Setup database
    let conn = setup_db();
    let tx = conn.unchecked_transaction()?;
    let txn = Txn::new(tx);

    // Create and register EdgeProvider registry
    let mut registry = EdgeProviderRegistry::new();
    registry.register::<UserNameIndex>();
    registry.register::<User>();
    registry.register::<Post>();
    registry.register::<Comment>();

    println!(
        "Registered EdgeProviders for UserNameIndex, User, Post, and Comment\n"
    );

    // Create the UserNameIndex singleton first (used for username uniqueness)
    println!("Creating UserNameIndex singleton for username uniqueness...");
    let username_index = UserNameIndex::new();
    let username_index_id = txn.create(username_index)?;
    println!("  Created UserNameIndex (id={})", username_index_id);

    // Create entities
    println!("\nCreating entities...");
    let user1 = User::new(
        "alice".to_string(),
        "alice@example.com".to_string(),
        username_index_id,
    );
    let user1_id = txn.create(user1)?;
    println!("  Created User (id={}): alice", user1_id);

    let user2 = User::new(
        "bob".to_string(),
        "bob@example.com".to_string(),
        username_index_id,
    );
    let user2_id = txn.create(user2)?;
    println!("  Created User (id={}): bob", user2_id);

    let post = Post::new(
        "Hello World".to_string(),
        "This is my first post".to_string(),
        user1_id,
        vec![user2_id], // Using user2_id as a tag for demonstration
    );
    let post_id = txn.create(post)?;
    println!("  Created Post (id={}): Hello World", post_id);

    let comment = Comment::new("Great post!".to_string(), post_id, user2_id);
    let comment_id = txn.create(comment)?;
    println!("  Created Comment (id={}): Great post!", comment_id);

    println!("\n=== Enumerating Edges from Box<dyn Ent> ===\n");

    // Retrieve entities as Box<dyn Ent> and enumerate their edges.
    // Note: For User, the UniqueUsernameDraft check() will fail because
    // it validates uniqueness, and the edge already exists (finding itself).
    // This is expected - EdgeProvider::draft().check() is designed for creation,
    // not enumeration. For audit purposes, use AuditEntEdges trait instead.
    let entities: Vec<Id> =
        vec![username_index_id, user1_id, user2_id, post_id, comment_id];

    for entity_id in entities {
        let ent = txn.get(entity_id)?.expect("Entity should exist");

        println!("Entity ID {}: {:?}", entity_id, ent.typetag_name());

        // Enumerate edges using the registry
        match registry.enumerate_edges(&ent, &txn) {
            Ok(edges) => {
                if edges.is_empty() {
                    println!("  No edges (IncomingEdgeProvider returns none)");
                } else {
                    println!("  Incoming edges (entity is destination):");
                    for edge in edges {
                        let edge_type = String::from_utf8_lossy(&edge.sort_key);
                        println!(
                            "    {} --[{}]--> {}",
                            edge.source, edge_type, edge.dest
                        );
                    }
                }
            }
            Err(e) => {
                // For User: uniqueness check fails because edge already exists
                println!("  Edge draft validation failed (expected for unique constraints): {}", e);
            }
        }
        println!();
    }

    println!("=== Testing Username Uniqueness Constraint ===\n");

    // Try to create a user with duplicate username (should fail validation)
    println!("Attempting to create User with duplicate username 'alice'...");
    let duplicate_user = User::new(
        "alice".to_string(),
        "alice2@example.com".to_string(),
        username_index_id,
    );

    match txn.create(duplicate_user) {
        Ok(_) => {
            println!("  ERROR: Should have failed due to duplicate username!")
        }
        Err(e) => println!("  Uniqueness constraint correctly failed: {}", e),
    }

    // Verify that creating a user with a different username still works
    println!("\nAttempting to create User with unique username 'charlie'...");
    let charlie = User::new(
        "charlie".to_string(),
        "charlie@example.com".to_string(),
        username_index_id,
    );
    match txn.create(charlie) {
        Ok(charlie_id) => {
            println!("  Successfully created User (id={}): charlie", charlie_id)
        }
        Err(e) => println!("  ERROR: Should have succeeded: {}", e),
    }

    println!("\n=== Querying Username Index Edges ===\n");

    // Query edges from the UserNameIndex to see all usernames
    println!("Edges from UserNameIndex (id={}):", username_index_id);
    let username_result =
        txn.find_edges(username_index_id, EdgeQuery::asc(&[]))?;
    for edge in &username_result.edges {
        let username = String::from_utf8_lossy(&edge.sort_key);
        println!("  {} --[{}]--> User {}", edge.source, username, edge.dest);
    }

    println!("\n=== Validating Edges with Transactions ===\n");

    // Try to create a post with invalid author (should fail validation)
    println!("Attempting to create Post with non-existent author...");
    let invalid_post = Post::new(
        "Invalid Post".to_string(),
        "This should fail".to_string(),
        9999, // Non-existent user ID
        vec![],
    );

    match txn.create(invalid_post) {
        Ok(_) => println!("  ERROR: Should have failed!"),
        Err(e) => println!("  Validation correctly failed: {}", e),
    }

    println!("\n=== Querying Created Edges ===\n");

    // Query outgoing edges from Post (source=Post)
    // Note: With IncomingEdgeProvider, Post's edges are incoming (dest=Post),
    // so querying by source=Post shows outgoing edges like has_comment.
    println!("Outgoing edges from Post (source={}):", post_id);
    let post_outgoing = txn.find_edges(post_id, EdgeQuery::asc(&[]))?;
    if post_outgoing.edges.is_empty() {
        println!("  (none - Post has incoming edges, not outgoing)");
    }
    for edge in &post_outgoing.edges {
        let edge_type = String::from_utf8_lossy(&edge.sort_key);
        println!("  {} --[{}]--> {}", edge.source, edge_type, edge.dest);
    }

    // Query incoming edges to Post (dest=Post) using AdminEdgeByDest
    println!("\nIncoming edges to Post (dest={}):", post_id);
    let post_incoming = txn.find_edges_by_dest(post_id)?;
    for edge in &post_incoming {
        let edge_type = String::from_utf8_lossy(&edge.sort_key);
        println!("  {} --[{}]--> {}", edge.source, edge_type, edge.dest);
    }

    println!("\nIncoming edges to Comment (dest={}):", comment_id);
    let comment_incoming = txn.find_edges_by_dest(comment_id)?;
    for edge in &comment_incoming {
        let edge_type = String::from_utf8_lossy(&edge.sort_key);
        println!("  {} --[{}]--> {}", edge.source, edge_type, edge.dest);
    }

    println!("\n=== Auditing Entities via AuditEntEdges Trait ===\n");

    // The AuditEntEdges trait provides a generalized audit mechanism that:
    // 1. Finds all edges whose dest is the entity id
    // 2. Removes those edges (within a transaction)
    // 3. Drafts edges via EdgeProvider (like a new entity)
    // 4. Compares existing vs drafted edges
    // 5. Drops the transaction (not committed) - database unchanged

    // Commit the current transaction first
    txn.commit()?;
    println!("Main transaction committed.\n");

    // Audit User entity using AuditEntEdges trait
    println!(
        "Auditing user 'alice' (id={}) using AuditEntEdges trait...",
        user1_id
    );
    {
        let audit_tx = conn.unchecked_transaction()?;
        let audit_txn = Txn::new(audit_tx);

        match audit_txn.audit_ent_edges::<User>(user1_id) {
            Ok(()) => println!("  ✓ User edges are valid!"),
            Err(e) => println!("  ✗ User edge audit failed: {}", e),
        }
        // Transaction is dropped here (not committed), database unchanged
    }

    // Audit Post entity
    println!(
        "\nAuditing post 'Hello World' (id={}) using AuditEntEdges trait...",
        post_id
    );
    {
        let audit_tx = conn.unchecked_transaction()?;
        let audit_txn = Txn::new(audit_tx);

        match audit_txn.audit_ent_edges::<Post>(post_id) {
            Ok(()) => println!("  ✓ Post edges are valid!"),
            Err(e) => println!("  ✗ Post edge audit failed: {}", e),
        }
    }

    // Audit Comment entity
    println!(
        "\nAuditing comment (id={}) using AuditEntEdges trait...",
        comment_id
    );
    {
        let audit_tx = conn.unchecked_transaction()?;
        let audit_txn = Txn::new(audit_tx);

        match audit_txn.audit_ent_edges::<Comment>(comment_id) {
            Ok(()) => println!("  ✓ Comment edges are valid!"),
            Err(e) => println!("  ✗ Comment edge audit failed: {}", e),
        }
    }

    // Verify database is unchanged by querying edges again
    println!("\nVerification: Database unchanged after audits");
    {
        let verify_tx = conn.unchecked_transaction()?;
        let verify_txn = Txn::new(verify_tx);
        let user_result =
            verify_txn.find_edges(username_index_id, EdgeQuery::asc(&[]))?;
        println!("  UserNameIndex edges: {} (unchanged)", user_result.edges.len());
    }

    println!("\n=== Audit Complete ===");

    Ok(())
}
