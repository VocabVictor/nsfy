use super::*;

fn change(id: &str, status: MessageStatus) -> StateChange {
    StateChange {
        id: id.to_string(),
        status,
        updated_at: 0,
    }
}

#[test]
fn state_round_trips_and_latest_value_wins() {
    let store = StateStore::open(":memory:").unwrap();
    store
        .record("alerts", &[change("one", MessageStatus::Read)], 10)
        .unwrap();
    store
        .record("alerts", &[change("one", MessageStatus::Trash)], 10)
        .unwrap();
    let loaded = store.load("alerts", 10).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].status, MessageStatus::Trash);
    assert!(loaded[0].updated_at > 0);
}

#[test]
fn state_retention_is_per_topic() {
    let store = StateStore::open(":memory:").unwrap();
    for index in 0..4 {
        store
            .record("a", &[change(&format!("a{index}"), MessageStatus::Read)], 2)
            .unwrap();
        store
            .record(
                "b",
                &[change(&format!("b{index}"), MessageStatus::Trash)],
                5,
            )
            .unwrap();
    }
    assert_eq!(store.load("a", 10).unwrap().len(), 2);
    assert_eq!(store.load("b", 10).unwrap().len(), 4);
}

#[test]
fn validates_batch_and_message_ids() {
    assert!(validate_request(&StateRequest { updates: vec![] }).is_err());
    assert!(validate_request(&StateRequest {
        updates: vec![change("bad\nid", MessageStatus::Read)]
    })
    .is_err());
    assert!(validate_request(&StateRequest {
        updates: vec![change("valid-id", MessageStatus::Purged)]
    })
    .is_ok());
}
