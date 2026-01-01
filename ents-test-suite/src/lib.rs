mod test_entity;

pub use test_entity::{Post, Tag, TestEntity, User};

use ents::{EdgeQuery, EntExt, Id, QueryEdge, Transactional};

pub trait TestCaseRunner {
    type Tx: Transactional;

    fn execute<F, R>(&mut self, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(Self::Tx) -> anyhow::Result<R>;
}

pub trait TestSuiteRunner: Clone {
    type CaseRunner: TestCaseRunner;

    fn create(&self) -> anyhow::Result<Self::CaseRunner>;
}

pub fn test_basic_create<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing basic create...");

    let mut runner1 = r.create()?;
    let id = runner1.execute(|txn| {
        let entity = TestEntity::new("test_create".to_string(), 42);
        let id = txn.create(entity)?;
        txn.commit()?;
        Ok(id)
    })?;

    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let retrieved = txn.get(id)?;
        match retrieved {
            Some(ent) => {
                let test_ent = ent
                    .as_ent::<TestEntity>()
                    .ok_or_else(|| anyhow::anyhow!("Entity is not TestEntity"))?;
                assert_eq!(test_ent.name, "test_create");
                assert_eq!(test_ent.value, 42);
                assert_eq!(test_ent.id, id);
            }
            None => return Err(anyhow::anyhow!("Entity not found after creation")),
        }
        txn.commit()?;
        Ok(())
    })
}

pub fn test_relationships<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing relationships (User-Post-Tag)...");

    let mut runner = r.create()?;
    runner.execute(|txn| {
        // Create a user
        let user = User::new("johndoe".to_string(), "john@example.com".to_string());
        let user_id = txn.create(user)?;

        // Create some tags
        let tag1 = Tag::new("rust".to_string(), "#ff6b6b".to_string());
        let tag1_id = txn.create(tag1)?;

        let tag2 = Tag::new("programming".to_string(), "#4ecdc4".to_string());
        let tag2_id = txn.create(tag2)?;

        let tag3 = Tag::new("tutorial".to_string(), "#45b7d1".to_string());
        let tag3_id = txn.create(tag3)?;

        // Create a post with the user as author and tags
        let post = Post::new(
            "Learning Rust".to_string(),
            "This is a comprehensive guide to Rust programming".to_string(),
            user_id,
            vec![tag1_id, tag2_id, tag3_id],
        );
        let post_id = txn.create(post)?;

        txn.commit()?;

        // Now query the relationships
        let mut runner2 = r.create()?;
        runner2.execute(|txn| {
            // Find the post's author
            let author_edges = txn.find_edges(post_id, EdgeQuery::asc(&[b"author"]))?;
            assert_eq!(author_edges.len(), 1, "Post should have exactly one author");
            assert_eq!(
                author_edges[0].dest, user_id,
                "Author edge should point to the correct user"
            );

            // Find the post's tags
            let tag_edges = txn.find_edges(post_id, EdgeQuery::asc(&[b"tag"]))?;
            assert_eq!(tag_edges.len(), 3, "Post should have exactly three tags");

            let mut tag_ids: Vec<Id> = tag_edges.iter().map(|e| e.dest).collect();
            tag_ids.sort();
            let expected_tags = vec![tag1_id, tag2_id, tag3_id];
            assert_eq!(
                tag_ids, expected_tags,
                "Tag edges should point to the correct tags"
            );

            // Verify we can retrieve the entities
            let retrieved_user = txn.get(user_id)?;
            match retrieved_user {
                Some(ent) => {
                    let user = ent
                        .as_ent::<User>()
                        .ok_or_else(|| anyhow::anyhow!("Entity is not User"))?;
                    assert_eq!(user.username, "johndoe");
                    assert_eq!(user.email, "john@example.com");
                }
                None => return Err(anyhow::anyhow!("User not found")),
            }

            let retrieved_post = txn.get(post_id)?;
            match retrieved_post {
                Some(ent) => {
                    let post = ent
                        .as_ent::<Post>()
                        .ok_or_else(|| anyhow::anyhow!("Entity is not Post"))?;
                    assert_eq!(post.title, "Learning Rust");
                    assert_eq!(post.author_id, user_id);
                    assert_eq!(post.tag_ids, vec![tag1_id, tag2_id, tag3_id]);
                }
                None => return Err(anyhow::anyhow!("Post not found")),
            }

            txn.commit()?;
            Ok(())
        })
    })?;

    Ok(())
}

pub fn run_all_tests<R: TestSuiteRunner + Clone>(runner: R) -> anyhow::Result<()> {
    println!("Running all test cases...");

    test_basic_create(&runner)?;
    test_basic_read(&runner)?;
    test_basic_update(&runner)?;
    test_basic_delete(&runner)?;
    test_error_handling(&runner)?;
    test_multiple_entities(&runner)?;
    test_relationships(&runner)?;

    println!("All tests passed!");
    Ok(())
}
pub fn test_basic_read<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing basic read...");

    let mut runner1 = r.create()?;
    let id = runner1.execute(|txn| {
        let entity = TestEntity::new("test_read".to_string(), 100);
        let id = txn.create(entity)?;
        txn.commit()?;
        Ok(id)
    })?;

    // Test reading
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let retrieved = txn.get(id)?;
        match retrieved {
            Some(ent) => {
                let test_ent = ent
                    .as_ent::<TestEntity>()
                    .ok_or_else(|| anyhow::anyhow!("Entity is not TestEntity"))?;
                assert_eq!(test_ent.name, "test_read");
                assert_eq!(test_ent.value, 100);
            }
            None => return Err(anyhow::anyhow!("Entity not found")),
        }

        // Test non-existent
        let non_existent = txn.get(999999)?;
        assert!(
            non_existent.is_none(),
            "Non-existent entity should return None"
        );

        txn.commit()?;
        Ok(())
    })
}

pub fn test_basic_update<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing basic update...");

    let mut runner1 = r.create()?;
    let id = runner1.execute(|txn| {
        let entity = TestEntity::new("test_update".to_string(), 50);
        let id = txn.create(entity)?;
        txn.commit()?;
        Ok(id)
    })?;

    // Update - get the entity and update it
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let retrieved = txn.get(id)?;
        match retrieved {
            Some(ent) => {
                if let Some(concrete_ent) = ent.downcast_ent::<TestEntity>() {
                    // Now concrete_ent is Box<TestEntity>, which implements BorrowMut<TestEntity>
                    let result = txn.update(concrete_ent, |e: &mut TestEntity| {
                        e.value = 75;
                        e.name = "updated_name".to_string();
                    })?;
                    assert!(result, "Update should succeed");
                } else {
                    return Err(anyhow::anyhow!("Entity is not TestEntity"));
                }
            }
            None => return Err(anyhow::anyhow!("Entity not found for update")),
        }
        txn.commit()?;
        Ok(())
    })?;

    // Verify
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        let retrieved = txn.get(id)?;
        match retrieved {
            Some(ent) => {
                let test_ent = ent
                    .as_ent::<TestEntity>()
                    .ok_or_else(|| anyhow::anyhow!("Entity is not TestEntity"))?;
                assert_eq!(test_ent.name, "updated_name");
                assert_eq!(test_ent.value, 75);
            }
            None => return Err(anyhow::anyhow!("Entity not found after update")),
        }
        txn.commit()?;
        Ok(())
    })
}

pub fn test_basic_delete<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing basic delete...");

    let mut runner1 = r.create()?;
    let id = runner1.execute(|txn| {
        let entity = TestEntity::new("test_delete".to_string(), 200);
        let id = txn.create(entity)?;
        txn.commit()?;
        Ok(id)
    })?;

    // Verify entity exists
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let retrieved = txn.get(id)?;
        assert!(retrieved.is_some(), "Entity should exist before delete");
        txn.commit()?;
        Ok(())
    })?;

    // Delete
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        txn.delete::<TestEntity>(id)?;
        txn.commit()?;
        Ok(())
    })?;

    // Verify entity is gone
    let mut runner4 = r.create()?;
    runner4.execute(|txn| {
        let retrieved = txn.get(id)?;
        assert!(retrieved.is_none(), "Entity should not exist after delete");
        txn.commit()?;
        Ok(())
    })
}

pub fn test_error_handling<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing error handling...");

    let mut runner = r.create()?;
    runner.execute(|txn| {
        // Test updating non-existent entity
        let non_existent_id = 999999;
        let retrieved = txn.get(non_existent_id)?;
        assert!(
            retrieved.is_none(),
            "Non-existent entity should return None"
        );

        // Test deleting non-existent entity should not error
        // (depending on implementation, this might or might not error)
        let _ = txn.delete::<TestEntity>(non_existent_id);

        txn.commit()?;
        Ok(())
    })
}

pub fn test_multiple_entities<R: TestSuiteRunner>(r: &R) -> anyhow::Result<()> {
    println!("  Testing multiple entities...");

    let mut runner1 = r.create()?;
    let ids = runner1.execute(|txn| {
        let mut ids = Vec::new();
        for i in 0..5 {
            let entity = TestEntity::new(format!("test_multi_{}", i), i * 10);
            let id = txn.create(entity)?;
            ids.push(id);
        }
        txn.commit()?;
        Ok(ids)
    })?;

    // Verify all entities exist and have correct data
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        for (i, &id) in ids.iter().enumerate() {
            let retrieved = txn.get(id)?;
            match retrieved {
                Some(ent) => {
                    let test_ent = ent
                        .as_ent::<TestEntity>()
                        .ok_or_else(|| anyhow::anyhow!("Entity is not TestEntity"))?;
                    assert_eq!(test_ent.name, format!("test_multi_{}", i));
                    assert_eq!(test_ent.value, i as i32 * 10);
                }
                None => return Err(anyhow::anyhow!("Entity {} not found", id)),
            }
        }
        txn.commit()?;
        Ok(())
    })
}
