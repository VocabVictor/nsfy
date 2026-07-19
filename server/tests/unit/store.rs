use super::*;
use crate::message::Message;

fn msg(id: &str, time: i64, body: &str) -> Message {
    Message {
        id: id.to_string(),
        time,
        title: "t".to_string(),
        message: body.to_string(),
        priority: 3,
        tags: vec!["tag1".to_string()],
        category: vec!["ops".to_string(), "backup".to_string()],
        popup: true,
        bypass_dnd: true,
    }
}

#[test]
fn insert_and_prune_keeps_only_the_most_recent_n_per_topic() {
    let store = Store::open(":memory:").unwrap();
    for i in 0..5 {
        store
            .insert_and_prune("alerts", &msg(&format!("id{i}"), i, "body"), 3)
            .unwrap();
    }
    let loaded = store.load_all(10).unwrap();
    assert_eq!(loaded.len(), 1);
    let (topic, msgs) = &loaded[0];
    assert_eq!(topic, "alerts");
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].id, "id2");
    assert_eq!(msgs[2].id, "id4");
}

#[test]
fn load_all_round_trips_fields_including_tags() {
    let store = Store::open(":memory:").unwrap();
    store
        .insert_and_prune("t", &msg("a", 1, "hello"), 10)
        .unwrap();
    let loaded = store.load_all(10).unwrap();
    let (_, msgs) = &loaded[0];
    assert_eq!(msgs[0].message, "hello");
    assert_eq!(msgs[0].tags, vec!["tag1".to_string()]);
    assert_eq!(
        msgs[0].category,
        vec!["ops".to_string(), "backup".to_string()]
    );
    assert!(msgs[0].popup);
    assert!(msgs[0].bypass_dnd);
}

#[test]
fn separate_topics_are_pruned_independently() {
    let store = Store::open(":memory:").unwrap();
    for i in 0..3 {
        store
            .insert_and_prune("a", &msg(&format!("a{i}"), i, "x"), 1)
            .unwrap();
        store
            .insert_and_prune("b", &msg(&format!("b{i}"), i, "x"), 5)
            .unwrap();
    }
    let loaded: std::collections::HashMap<_, _> = store.load_all(10).unwrap().into_iter().collect();
    assert_eq!(loaded["a"].len(), 1);
    assert_eq!(loaded["b"].len(), 3);
}

#[test]
fn one_batch_prunes_each_topic_after_all_inserts() {
    let store = Store::open(":memory:").unwrap();
    let messages: Vec<_> = (0..5).map(|i| msg(&format!("id{i}"), i, "body")).collect();
    let writes: Vec<_> = messages
        .iter()
        .map(|message| ("alerts", message, 3))
        .collect();
    store.insert_and_prune_batch(&writes).unwrap();
    let loaded = store.load_all(10).unwrap();
    assert_eq!(loaded[0].1.len(), 3);
    assert_eq!(loaded[0].1[0].id, "id2");
    assert_eq!(loaded[0].1[2].id, "id4");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn queued_concurrent_records_are_acknowledged_after_commit() {
    let store = Arc::new(Store::open(":memory:").unwrap());
    let persistence = Persistence::sqlite(Arc::clone(&store)).unwrap();
    let tasks: Vec<_> = (0..32)
        .map(|i| {
            let persistence = persistence.clone();
            tokio::spawn(async move {
                persistence
                    .record("alerts", &msg(&format!("id{i}"), i, "body"), 64)
                    .await
            })
        })
        .collect();
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    assert_eq!(store.load_all(64).unwrap()[0].1.len(), 32);
}
