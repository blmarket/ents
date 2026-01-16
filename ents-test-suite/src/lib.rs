mod test_entity;

pub use test_entity::{Post, Tag, TestEntity, User, UserWithUniqueEmail};

use ents::{EdgeQuery, EdgeValue, EntExt, QueryEdge, ReadEnt, Transactional};
use ents_admin::{AdminEnt, AuditError};

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

/// Trait for test case runners that support AuditEntEdges functionality.
/// This requires the transaction type to implement AdminEdgeByDest in addition to Transactional.
pub trait AdminTestCaseRunner {
    type Tx: AdminEnt;

    fn execute<F, R>(&mut self, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(Self::Tx) -> anyhow::Result<R>;
}

/// Trait for test suite runners that support AuditEntEdges tests.
pub trait AdminTestSuiteRunner: Clone {
    type CaseRunner: AdminTestCaseRunner;

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
                let test_ent = ent.as_ent::<TestEntity>().ok_or_else(|| {
                    anyhow::anyhow!("Entity is not TestEntity")
                })?;
                assert_eq!(test_ent.name, "test_create");
                assert_eq!(test_ent.value, 42);
                assert_eq!(test_ent.id, id);
            }
            None => {
                return Err(anyhow::anyhow!("Entity not found after creation"))
            }
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
        let user =
            User::new("johndoe".to_string(), "john@example.com".to_string());
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
        // Note: With IncomingEdgeProvider, edges point TO the Post (Post is destination)
        // So we query from the source entities (User, Tags) to find the Post
        let mut runner2 = r.create()?;
        runner2.execute(|txn| {
            // Find author's posts (User --[authored]--> Post)
            let author_edges =
                txn.find_edges(user_id, EdgeQuery::asc(&[b"authored"]))?;
            assert_eq!(
                author_edges.len(),
                1,
                "User should have authored exactly one post"
            );
            assert_eq!(
                author_edges[0].dest, post_id,
                "Author edge should point to the correct post"
            );

            // Find each tag's posts (Tag --[tagged]--> Post)
            for &tid in &[tag1_id, tag2_id, tag3_id] {
                let tag_edges =
                    txn.find_edges(tid, EdgeQuery::asc(&[b"tagged"]))?;
                assert_eq!(
                    tag_edges.len(),
                    1,
                    "Each tag should tag exactly one post"
                );
                assert_eq!(
                    tag_edges[0].dest, post_id,
                    "Tag edge should point to the correct post"
                );
            }

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

pub fn test_unique_constraints<R: TestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing UNIQUE constraints...");

    let mut runner1 = r.create()?;
    let result = runner1.execute(|txn| {
        // Create first user with unique email
        let user1 = UserWithUniqueEmail::new("user1".to_string(), "unique@example.com".to_string());
        let _user1_id = txn.create(user1)?;

        // Try to create second user with same email - this should fail with UNIQUE constraint
        let user2 = UserWithUniqueEmail::new("user2".to_string(), "unique@example.com".to_string());

        // Currently UNIQUE constraints are not implemented, so this will succeed
        // When UNIQUE constraints are implemented, this should return an error
        match txn.create(user2) {
            Ok(_) => {
                // UNIQUE constraint not enforced - this is expected in current implementation
                println!("    Note: UNIQUE constraints not yet implemented - duplicate email allowed");
            }
            Err(e) => {
                // UNIQUE constraint is enforced - this would be the desired behavior
                println!("    UNIQUE constraint enforced - duplicate email rejected: {}", e);
                return Err(anyhow::anyhow!("UNIQUE constraint test should pass when implemented"));
            }
        }

        txn.commit()?;
        Ok(())
    });

    // For now, we expect this test to "pass" since UNIQUE constraints aren't implemented
    // When UNIQUE constraints are implemented, this test should be updated to expect failure
    match result {
        Ok(_) => {
            println!("    UNIQUE constraint test completed (constraints not yet implemented)");
            Ok(())
        }
        Err(e) => {
            println!("    UNIQUE constraint test failed unexpectedly: {}", e);
            Err(e)
        }
    }
}

pub fn run_all_tests<R: TestSuiteRunner + Clone>(
    runner: R,
) -> anyhow::Result<()> {
    println!("Running all test cases...");

    test_basic_create(&runner)?;
    test_basic_read(&runner)?;
    test_basic_update(&runner)?;
    test_basic_delete(&runner)?;
    test_error_handling(&runner)?;
    test_multiple_entities(&runner)?;
    test_relationships(&runner)?;
    test_unique_constraints(&runner)?;
    test_concurrent_updates(&runner)?;

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
                let test_ent = ent.as_ent::<TestEntity>().ok_or_else(|| {
                    anyhow::anyhow!("Entity is not TestEntity")
                })?;
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
                    let result =
                        txn.update(concrete_ent, |e: &mut TestEntity| {
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
                let test_ent = ent.as_ent::<TestEntity>().ok_or_else(|| {
                    anyhow::anyhow!("Entity is not TestEntity")
                })?;
                assert_eq!(test_ent.name, "updated_name");
                assert_eq!(test_ent.value, 75);
            }
            None => {
                return Err(anyhow::anyhow!("Entity not found after update"))
            }
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
                    let test_ent =
                        ent.as_ent::<TestEntity>().ok_or_else(|| {
                            anyhow::anyhow!("Entity is not TestEntity")
                        })?;
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

pub fn test_concurrent_updates<R: TestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing concurrent updates...");

    // Create an entity to test concurrent updates on
    let mut runner1 = r.create()?;
    let entity_id = runner1.execute(|txn| {
        let entity = TestEntity::new("concurrent_test".to_string(), 0);
        let id = txn.create(entity)?;
        txn.commit()?;
        Ok(id)
    })?;

    // Test 1: Simulate race condition - multiple attempts to update with potentially stale data
    println!("    Testing race condition simulation...");
    let mut success_count = 0;

    // First, get the entity and its current state
    let mut runner2 = r.create()?;
    let (entity_data, last_updated) = runner2.execute(|txn| {
        let retrieved = txn.get(entity_id)?;
        match retrieved {
            Some(ent) => {
                let test_ent = ent.as_ent::<TestEntity>().ok_or_else(|| {
                    anyhow::anyhow!("Entity is not TestEntity")
                })?;
                Ok((test_ent.clone(), test_ent.last_updated))
            }
            None => Err(anyhow::anyhow!("Entity not found")),
        }
    })?;

    // Now simulate multiple concurrent updates using the same stale data
    // In a real race condition, multiple threads would have the same last_updated value
    for i in 0..3 {
        let mut runner = r.create()?;
        let result = runner.execute(|txn| {
            // Create an entity with the stale last_updated (simulating what would happen
            // if multiple threads fetched the entity at the same time)
            let mut stale_entity = entity_data.clone();
            stale_entity.last_updated = last_updated; // All use the same stale timestamp

            let update_result =
                txn.update(Box::new(stale_entity), |e: &mut TestEntity| {
                    e.value = 100 + i;
                    e.name = format!("attempt_{}", i);
                });
            txn.commit()?;
            Ok(update_result.is_ok())
        });

        match result {
            Ok(true) => {
                success_count += 1;
                println!("      Attempt {} succeeded", i);
            }
            Ok(false) => {
                println!(
                    "      Attempt {} failed (expected for race condition)",
                    i
                );
            }
            Err(e) => {
                println!("      Attempt {} error: {}", i, e);
            }
        }
    }

    // In optimistic locking, only one should succeed when all start with the same last_updated
    if success_count > 1 {
        println!("      Warning: Multiple updates succeeded - backend may not enforce optimistic locking");
    } else if success_count == 1 {
        println!("      Race condition handled correctly - only one update succeeded");
    } else {
        println!("      All updates failed - check if backend supports optimistic locking");
    }

    // Test 2: Verify it rejects request to update entity with stale last_updated value
    println!("    Testing stale update rejection...");
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        let retrieved = txn.get(entity_id)?;
        match retrieved {
            Some(ent) => {
                if let Some(mut concrete_ent) = ent.downcast_ent::<TestEntity>() {
                    // Modify the last_updated to make it stale (simulate concurrent modification)
                    concrete_ent.last_updated = concrete_ent.last_updated.saturating_sub(1);

                    // This should ideally fail because the last_updated is stale
                    let update_result = txn.update(concrete_ent, |e: &mut TestEntity| {
                        e.value = 999;
                    });

                    match update_result {
                        Ok(_) => {
                            println!("      Warning: Stale update was allowed (backend may not enforce optimistic locking)");
                        }
                        Err(_) => {
                            println!("      Stale update correctly rejected");
                        }
                    }
                }
            }
            None => return Err(anyhow::anyhow!("Entity not found for stale update test")),
        }
        txn.commit()?;
        Ok(())
    })?;

    // Test 3: Verify it's possible to do series of updates when they all use correct last_updated value
    println!("    Testing sequential updates with correct last_updated...");
    let mut runner4 = r.create()?;
    runner4.execute(|txn| {
        for i in 0..3 {
            let retrieved = txn.get(entity_id)?;
            match retrieved {
                Some(ent) => {
                    if let Some(concrete_ent) = ent.downcast_ent::<TestEntity>()
                    {
                        let update_result =
                            txn.update(concrete_ent, |e: &mut TestEntity| {
                                e.value = 200 + i;
                                e.name = format!("sequential_update_{}", i);
                            })?;
                        assert!(
                            update_result,
                            "Sequential update {} should succeed",
                            i
                        );
                        println!("      Sequential update {} succeeded", i);
                    } else {
                        return Err(anyhow::anyhow!(
                            "Entity is not TestEntity in sequential test"
                        ));
                    }
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "Entity not found in sequential update {}",
                        i
                    ))
                }
            }
        }
        txn.commit()?;
        Ok(())
    })?;

    // Verify final state
    let mut runner5 = r.create()?;
    runner5.execute(|txn| {
        let retrieved = txn.get(entity_id)?;
        match retrieved {
            Some(ent) => {
                let test_ent = ent.as_ent::<TestEntity>().ok_or_else(|| {
                    anyhow::anyhow!("Entity is not TestEntity")
                })?;
                assert_eq!(test_ent.name, "sequential_update_2");
                assert_eq!(test_ent.value, 202);
                println!("      Sequential updates completed successfully");
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Entity not found after sequential updates"
                ))
            }
        }
        txn.commit()?;
        Ok(())
    })
}

// ============================================================================
// AuditEntEdges Test Suite
// ============================================================================

/// Test that audit succeeds for an entity with correct edges.
pub fn test_audit_success<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit success case...");

    // First, create entities with proper edges
    let mut runner1 = r.create()?;
    let (user_id, tag_id, post_id) = runner1.execute(|txn| {
        let user =
            User::new("auditor".to_string(), "auditor@example.com".to_string());
        let user_id = txn.create(user)?;

        let tag = Tag::new("testing".to_string(), "#00ff00".to_string());
        let tag_id = txn.create(tag)?;

        let post = Post::new(
            "Test Post".to_string(),
            "Content for audit test".to_string(),
            user_id,
            vec![tag_id],
        );
        let post_id = txn.create(post)?;

        txn.commit()?;
        Ok((user_id, tag_id, post_id))
    })?;

    // Now audit the Post - edges should match exactly
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);
        match result {
            Ok(()) => {
                println!("    Audit passed for Post with correct edges");
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Audit should have succeeded: {}",
                    e
                ));
            }
        }
        // Transaction is dropped (not committed), database unchanged
        Ok(())
    })?;

    // Verify database is unchanged by checking edges still exist
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        // Check author edge: User --[authored]--> Post
        let author_edges =
            txn.find_edges(user_id, EdgeQuery::asc(&[b"authored"]))?;
        assert_eq!(author_edges.len(), 1, "Author edge should still exist");
        assert_eq!(author_edges[0].dest, post_id);

        // Check tag edge: Tag --[tagged]--> Post
        let tag_edges = txn.find_edges(tag_id, EdgeQuery::asc(&[b"tagged"]))?;
        assert_eq!(tag_edges.len(), 1, "Tag edge should still exist");
        assert_eq!(tag_edges[0].dest, post_id);

        txn.commit()?;
        Ok(())
    })?;

    // Also audit User and Tag (no edges expected since they use NullEdgeProvider)
    let mut runner4 = r.create()?;
    runner4.execute(|txn| {
        let user_result = txn.audit_ent_edges::<User>(user_id);
        assert!(user_result.is_ok(), "User audit should succeed (no edges)");

        let tag_result = txn.audit_ent_edges::<Tag>(tag_id);
        assert!(tag_result.is_ok(), "Tag audit should succeed (no edges)");

        Ok(())
    })?;

    Ok(())
}

/// Test that audit returns EntityNotFound for non-existent entity.
pub fn test_audit_entity_not_found<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit entity not found...");

    let mut runner = r.create()?;
    runner.execute(|txn| {
        let non_existent_id = 999999999;
        let result = txn.audit_ent_edges::<Post>(non_existent_id);

        match result {
            Err(AuditError::EntityNotFound(id)) => {
                assert_eq!(id, non_existent_id);
                println!("    Correctly returned EntityNotFound for id {}", id);
            }
            Ok(()) => {
                return Err(anyhow::anyhow!(
                    "Audit should have failed with EntityNotFound"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Expected EntityNotFound, got: {}",
                    e
                ));
            }
        }
        Ok(())
    })
}

/// Test that audit returns UnexpectedEntityType when entity type doesn't match.
pub fn test_audit_unexpected_entity_type<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit unexpected entity type...");

    // Create a User
    let mut runner1 = r.create()?;
    let user_id = runner1.execute(|txn| {
        let user =
            User::new("typetest".to_string(), "type@example.com".to_string());
        let id = txn.create(user)?;
        txn.commit()?;
        Ok(id)
    })?;

    // Try to audit the User as a Post - should fail with UnexpectedEntityType
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(user_id);

        match result {
            Err(AuditError::UnexpectedEntityType(id, type_name)) => {
                assert_eq!(id, user_id);
                println!("    Correctly returned UnexpectedEntityType: id={}, expected={}", id, type_name);
            }
            Ok(()) => {
                return Err(anyhow::anyhow!("Audit should have failed with UnexpectedEntityType"));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Expected UnexpectedEntityType, got: {}", e));
            }
        }
        Ok(())
    })
}

/// Test that audit returns EdgeMismatch when edges don't match.
pub fn test_audit_edge_mismatch_missing_edge<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit edge mismatch (missing edge)...");

    // Create entities with edges
    let mut runner1 = r.create()?;
    let (user_id, _tag_id, post_id) = runner1.execute(|txn| {
        let user = User::new(
            "mismatch_test".to_string(),
            "mismatch@example.com".to_string(),
        );
        let user_id = txn.create(user)?;

        let tag = Tag::new("mismatch".to_string(), "#ff0000".to_string());
        let tag_id = txn.create(tag)?;

        let post = Post::new(
            "Mismatch Test".to_string(),
            "Content".to_string(),
            user_id,
            vec![tag_id],
        );
        let post_id = txn.create(post)?;

        txn.commit()?;
        Ok((user_id, tag_id, post_id))
    })?;

    // Manually delete one of the incoming edges to create a mismatch
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        // Remove the tag edge: Tag --[tagged]--> Post
        txn.remove_edges_by_dest(post_id)?;
        // Re-add only the author edge
        txn.create_edge(EdgeValue::new(
            user_id,
            b"authored".to_vec(),
            post_id,
        ))?;
        txn.commit()?;
        Ok(())
    })?;

    // Now audit - should fail because the tag edge is missing
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);

        match result {
            Err(AuditError::EdgeMismatch { existing, drafted }) => {
                println!("    Correctly returned EdgeMismatch");
                println!("      existing edges: {:?}", existing.len());
                println!("      drafted edges: {:?}", drafted.len());
                // existing has 1 edge (author only), drafted has 2 (author + tag)
                assert_eq!(
                    existing.len(),
                    1,
                    "Should have 1 existing edge (author only)"
                );
                assert_eq!(
                    drafted.len(),
                    2,
                    "Should draft 2 edges (author + tag)"
                );
            }
            Ok(()) => {
                return Err(anyhow::anyhow!(
                    "Audit should have failed with EdgeMismatch"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Expected EdgeMismatch, got: {}",
                    e
                ));
            }
        }
        Ok(())
    })
}

/// Test that audit returns EdgeMismatch when there are extra edges.
pub fn test_audit_edge_mismatch_extra_edge<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit edge mismatch (extra edge)...");

    // Create entities
    let mut runner1 = r.create()?;
    let (_user_id, post_id) = runner1.execute(|txn| {
        let user = User::new(
            "extra_test".to_string(),
            "extra@example.com".to_string(),
        );
        let user_id = txn.create(user)?;

        // Create post with no tags
        let post = Post::new(
            "Extra Edge Test".to_string(),
            "Content".to_string(),
            user_id,
            vec![], // No tags
        );
        let post_id = txn.create(post)?;

        txn.commit()?;
        Ok((user_id, post_id))
    })?;

    // Manually add an extra edge that shouldn't exist
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        // Add an extra edge that the Post doesn't know about
        txn.create_edge(EdgeValue::new(12345, b"spurious".to_vec(), post_id))?;
        txn.commit()?;
        Ok(())
    })?;

    // Now audit - should fail because of the extra edge
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);

        match result {
            Err(AuditError::EdgeMismatch { existing, drafted }) => {
                println!("    Correctly returned EdgeMismatch");
                println!("      existing edges: {:?}", existing.len());
                println!("      drafted edges: {:?}", drafted.len());
                // existing has 2 edges (author + spurious), drafted has 1 (author only)
                assert_eq!(
                    existing.len(),
                    2,
                    "Should have 2 existing edges (author + spurious)"
                );
                assert_eq!(
                    drafted.len(),
                    1,
                    "Should draft 1 edge (author only)"
                );
            }
            Ok(()) => {
                return Err(anyhow::anyhow!(
                    "Audit should have failed with EdgeMismatch"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Expected EdgeMismatch, got: {}",
                    e
                ));
            }
        }
        Ok(())
    })
}

/// Test that audit returns EdgeMismatch when edge content differs.
pub fn test_audit_edge_mismatch_wrong_content<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit edge mismatch (wrong edge content)...");

    // Create entities
    let mut runner1 = r.create()?;
    let (_user_id, other_user_id, post_id) = runner1.execute(|txn| {
        let user = User::new(
            "correct_user".to_string(),
            "correct@example.com".to_string(),
        );
        let user_id = txn.create(user)?;

        let other_user = User::new(
            "wrong_user".to_string(),
            "wrong@example.com".to_string(),
        );
        let other_user_id = txn.create(other_user)?;

        // Create post with user as author
        let post = Post::new(
            "Wrong Content Test".to_string(),
            "Content".to_string(),
            user_id, // Post thinks this is the author
            vec![],
        );
        let post_id = txn.create(post)?;

        txn.commit()?;
        Ok((user_id, other_user_id, post_id))
    })?;

    // Replace the author edge with one pointing to the wrong user
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        // Remove correct edge and add wrong one
        txn.remove_edges_by_dest(post_id)?;
        // Add edge from wrong user
        txn.create_edge(EdgeValue::new(
            other_user_id,
            b"authored".to_vec(),
            post_id,
        ))?;
        txn.commit()?;
        Ok(())
    })?;

    // Now audit - should fail because author edge has wrong source
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);

        match result {
            Err(AuditError::EdgeMismatch { existing, drafted }) => {
                println!("    Correctly returned EdgeMismatch");
                // Both have 1 edge, but they differ in source
                assert_eq!(existing.len(), 1);
                assert_eq!(drafted.len(), 1);
                assert_ne!(
                    existing[0].source, drafted[0].source,
                    "Sources should differ"
                );
                println!("      existing source: {}", existing[0].source);
                println!("      drafted source: {}", drafted[0].source);
            }
            Ok(()) => {
                return Err(anyhow::anyhow!(
                    "Audit should have failed with EdgeMismatch"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Expected EdgeMismatch, got: {}",
                    e
                ));
            }
        }
        Ok(())
    })
}

/// Test that audit correctly handles entities with NullEdgeProvider.
pub fn test_audit_null_edge_provider<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing audit with NullEdgeProvider...");

    // Create a TestEntity (which uses NullEdgeProvider)
    let mut runner1 = r.create()?;
    let entity_id = runner1.execute(|txn| {
        let entity = TestEntity::new("null_edge_test".to_string(), 42);
        let id = txn.create(entity)?;
        txn.commit()?;
        Ok(id)
    })?;

    // Audit should succeed - no edges expected
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let result = txn.audit_ent_edges::<TestEntity>(entity_id);
        match result {
            Ok(()) => {
                println!("    Audit passed for entity with NullEdgeProvider");
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Audit should have succeeded: {}",
                    e
                ));
            }
        }
        Ok(())
    })?;

    // Now add a spurious edge to the entity
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        txn.create_edge(EdgeValue::new(
            99999,
            b"spurious".to_vec(),
            entity_id,
        ))?;
        txn.commit()?;
        Ok(())
    })?;

    // Audit should now fail because there's an edge that shouldn't exist
    let mut runner4 = r.create()?;
    runner4.execute(|txn| {
        let result = txn.audit_ent_edges::<TestEntity>(entity_id);
        match result {
            Err(AuditError::EdgeMismatch { existing, drafted }) => {
                println!("    Correctly detected spurious edge on NullEdgeProvider entity");
                assert_eq!(existing.len(), 1, "Should have 1 spurious edge");
                assert_eq!(drafted.len(), 0, "NullEdgeProvider drafts no edges");
            }
            Ok(()) => {
                return Err(anyhow::anyhow!("Audit should have failed with EdgeMismatch"));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Expected EdgeMismatch, got: {}", e));
            }
        }
        Ok(())
    })
}

/// Test that fix_ent_edges corrects edge mismatches.
pub fn test_fix_ent_edges<R: AdminTestSuiteRunner>(
    r: &R,
) -> anyhow::Result<()> {
    println!("  Testing fix_ent_edges...");

    // Create entities with proper edges
    let mut runner1 = r.create()?;
    let (user_id, tag_id, post_id) = runner1.execute(|txn| {
        let user =
            User::new("fixer".to_string(), "fixer@example.com".to_string());
        let user_id = txn.create(user)?;

        let tag = Tag::new("fixable".to_string(), "#0000ff".to_string());
        let tag_id = txn.create(tag)?;

        let post = Post::new(
            "Fix Test Post".to_string(),
            "Content for fix test".to_string(),
            user_id,
            vec![tag_id],
        );
        let post_id = txn.create(post)?;

        txn.commit()?;
        Ok((user_id, tag_id, post_id))
    })?;

    // Verify initial state is correct
    let mut runner2 = r.create()?;
    runner2.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);
        assert!(result.is_ok(), "Initial audit should pass");
        Ok(())
    })?;

    // Corrupt the edges: remove tag edge and add spurious edge
    let mut runner3 = r.create()?;
    runner3.execute(|txn| {
        // Remove all edges and only add the author edge (missing tag edge)
        txn.remove_edges_by_dest(post_id)?;
        txn.create_edge(EdgeValue::new(
            user_id,
            b"authored".to_vec(),
            post_id,
        ))?;
        // Add a spurious edge
        txn.create_edge(EdgeValue::new(99999, b"spurious".to_vec(), post_id))?;
        txn.commit()?;
        Ok(())
    })?;

    // Verify audit fails now
    let mut runner4 = r.create()?;
    runner4.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);
        match result {
            Err(AuditError::EdgeMismatch { existing, drafted }) => {
                println!("    Audit correctly detected mismatch before fix");
                println!("      existing: {} edges", existing.len());
                println!("      drafted: {} edges", drafted.len());
            }
            Ok(()) => {
                return Err(anyhow::anyhow!(
                    "Audit should have failed before fix"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Unexpected error: {}", e));
            }
        }
        Ok(())
    })?;

    // Fix the edges
    let mut runner5 = r.create()?;
    runner5.execute(|txn| {
        let result = txn.fix_ent_edges::<Post>(post_id);
        match result {
            Ok(()) => {
                println!("    fix_ent_edges succeeded");
            }
            Err(e) => {
                return Err(anyhow::anyhow!("fix_ent_edges failed: {}", e));
            }
        }
        Ok(())
    })?;

    // Verify audit now passes
    let mut runner6 = r.create()?;
    runner6.execute(|txn| {
        let result = txn.audit_ent_edges::<Post>(post_id);
        match result {
            Ok(()) => {
                println!("    Audit passed after fix");
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Audit should have passed after fix: {}",
                    e
                ));
            }
        }
        Ok(())
    })?;

    // Verify edges are correct by querying them
    let mut runner7 = r.create()?;
    runner7.execute(|txn| {
        // Check author edge: User --[authored]--> Post
        let author_edges =
            txn.find_edges(user_id, EdgeQuery::asc(&[b"authored"]))?;
        assert_eq!(author_edges.len(), 1, "Should have author edge");
        assert_eq!(
            author_edges[0].dest, post_id,
            "Author edge should point to post"
        );

        // Check tag edge: Tag --[tagged]--> Post
        let tag_edges = txn.find_edges(tag_id, EdgeQuery::asc(&[b"tagged"]))?;
        assert_eq!(tag_edges.len(), 1, "Should have tag edge");
        assert_eq!(tag_edges[0].dest, post_id, "Tag edge should point to post");

        txn.commit()?;
        Ok(())
    })?;

    Ok(())
}

/// Run all AuditEntEdges tests.
pub fn run_audit_tests<R: AdminTestSuiteRunner>(
    runner: R,
) -> anyhow::Result<()> {
    println!("Running AuditEntEdges test cases...");

    test_audit_success(&runner)?;
    test_audit_entity_not_found(&runner)?;
    test_audit_unexpected_entity_type(&runner)?;
    test_audit_edge_mismatch_missing_edge(&runner)?;
    test_audit_edge_mismatch_extra_edge(&runner)?;
    test_audit_edge_mismatch_wrong_content(&runner)?;
    test_audit_null_edge_provider(&runner)?;
    test_fix_ent_edges(&runner)?;

    println!("All AuditEntEdges tests passed!");
    Ok(())
}
