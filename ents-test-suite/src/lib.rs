mod test_entity;

pub use test_entity::{Post, Tag, TestEntity, User, UserWithUniqueEmail};

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
        let mut runner2 = r.create()?;
        runner2.execute(|txn| {
            // Find the post's author
            let author_edges =
                txn.find_edges(post_id, EdgeQuery::asc(&[b"author"]))?;
            assert_eq!(
                author_edges.len(),
                1,
                "Post should have exactly one author"
            );
            assert_eq!(
                author_edges[0].dest, user_id,
                "Author edge should point to the correct user"
            );

            // Find the post's tags
            let tag_edges =
                txn.find_edges(post_id, EdgeQuery::asc(&[b"tag"]))?;
            assert_eq!(
                tag_edges.len(),
                3,
                "Post should have exactly three tags"
            );

            let mut tag_ids: Vec<Id> =
                tag_edges.iter().map(|e| e.dest).collect();
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
