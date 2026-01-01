use ents::{
    DraftError, EdgeDraft, EdgeProvider, EdgeValue, Ent, EntMutationError, EntWithEdges, Id,
    NullEdgeProvider,
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
            .as_secs();
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
            .as_secs();
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
            .as_secs();
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
            .as_secs();
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
