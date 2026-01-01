use ents::{
    DraftError, EdgeDraft, EdgeProvider, EdgeQuery, EdgeValue, Ent, EntMutationError, EntWithEdges,
    Id, NullEdgeProvider, Transactional,
};
use serde::{Deserialize, Serialize};

/// Simple test entity for basic CRUD operations
#[derive(Clone, Serialize, Deserialize)]
pub struct TestEntity {
    pub name: String,
    pub value: i32,
    pub id: Id,
    pub last_updated: u64,
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
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        Ok(())
    }
}

impl EntWithEdges for TestEntity {
    type EdgeProvider = NullEdgeProvider;
}

impl TestEntity {
    pub fn new(name: String, value: i32) -> Self {
        Self {
            name,
            value,
            id: 0,
            last_updated: 0,
        }
    }
}

/// User entity for testing relationships
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub email: String,
    pub id: Id,
    pub last_updated: u64,
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
            .as_micros() as u64;
        Ok(())
    }
}

impl EntWithEdges for User {
    type EdgeProvider = NullEdgeProvider;
}

impl User {
    pub fn new(username: String, email: String) -> Self {
        Self {
            username,
            email,
            id: 0,
            last_updated: 0,
        }
    }
}

/// User entity with unique email constraint for testing
#[derive(Clone, Serialize, Deserialize)]
pub struct UserWithUniqueEmail {
    pub username: String,
    pub email: String,
    pub id: Id,
    pub last_updated: u64,
}

#[typetag::serde]
impl Ent for UserWithUniqueEmail {
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

/// Edge draft for enforcing unique email constraint
#[derive(PartialEq)]
pub struct UniqueEmailDraft {
    pub user_id: Id,
    pub email: String,
}

impl EdgeDraft for UniqueEmailDraft {
    fn check<T: Transactional>(self, txn: &T) -> Result<Vec<EdgeValue>, DraftError> {
        // Check if any existing user has this email
        let existing_edges = txn
            .find_edges(0, EdgeQuery::asc(&[b"unique_email"]))?
            .into_iter()
            .filter(|_edge| {
                // In a real implementation, we'd need to check the email value
                // For now, this is a placeholder - UNIQUE constraints aren't fully implemented
                false // This would be replaced with actual uniqueness checking
            })
            .collect::<Vec<_>>();

        if !existing_edges.is_empty() {
            return Err(DraftError::ValidationFailed(format!(
                "Email '{}' is already taken",
                self.email
            )));
        }

        // Create the unique email edge
        Ok(vec![EdgeValue::new(
            0, // Use a special source ID for global constraints
            b"unique_email".to_vec(),
            self.user_id,
        )])
    }
}

/// Edge provider for UserWithUniqueEmail that enforces unique email
pub struct UserWithUniqueEmailEdgeProvider;

impl EdgeProvider<UserWithUniqueEmail> for UserWithUniqueEmailEdgeProvider {
    type Draft = UniqueEmailDraft;

    fn draft(ent: &UserWithUniqueEmail) -> Self::Draft {
        UniqueEmailDraft {
            user_id: ent.id,
            email: ent.email.clone(),
        }
    }
}

impl EntWithEdges for UserWithUniqueEmail {
    type EdgeProvider = UserWithUniqueEmailEdgeProvider;
}

impl UserWithUniqueEmail {
    pub fn new(username: String, email: String) -> Self {
        Self {
            username,
            email,
            id: 0,
            last_updated: 0,
        }
    }
}

/// Tag entity for testing relationships
#[derive(Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub color: String,
    pub id: Id,
    pub last_updated: u64,
}

#[typetag::serde]
impl Ent for Tag {
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

impl EntWithEdges for Tag {
    type EdgeProvider = NullEdgeProvider;
}

impl Tag {
    pub fn new(name: String, color: String) -> Self {
        Self {
            name,
            color,
            id: 0,
            last_updated: 0,
        }
    }
}

/// Post entity with relationships to User (author) and Tags
#[derive(Clone, Serialize, Deserialize)]
pub struct Post {
    pub title: String,
    pub content: String,
    pub author_id: Id,
    pub tag_ids: Vec<Id>,
    pub id: Id,
    pub last_updated: u64,
}

/// Edge draft for author relationship
#[derive(PartialEq)]
pub struct AuthorEdgeDraft {
    pub post_id: Id,
    pub author_id: Id,
}

impl EdgeDraft for AuthorEdgeDraft {
    fn check<T: ents::Transactional>(self, _txn: &T) -> Result<Vec<EdgeValue>, DraftError> {
        Ok(vec![EdgeValue::new(
            self.post_id,
            b"author".to_vec(),
            self.author_id,
        )])
    }
}

/// Edge draft for tag relationships
#[derive(PartialEq)]
pub struct TagsEdgeDraft {
    pub post_id: Id,
    pub tag_ids: Vec<Id>,
}

impl EdgeDraft for TagsEdgeDraft {
    fn check<T: ents::Transactional>(self, _txn: &T) -> Result<Vec<EdgeValue>, DraftError> {
        let mut edges = Vec::new();
        for tag_id in self.tag_ids {
            edges.push(EdgeValue::new(self.post_id, b"tag".to_vec(), tag_id));
        }
        Ok(edges)
    }
}

/// Edge provider for Post that creates both author and tag edges
pub struct PostEdgeProvider;

impl EdgeProvider<Post> for PostEdgeProvider {
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

impl EntWithEdges for Post {
    type EdgeProvider = PostEdgeProvider;
}

impl Post {
    pub fn new(title: String, content: String, author_id: Id, tag_ids: Vec<Id>) -> Self {
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
