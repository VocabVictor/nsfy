use super::*;

#[test]
fn topic_exists_reflects_creation_without_creating_on_check() {
    let ps = PubSub::new(10, 32, 10);
    assert!(!ps.topic_exists("alerts"));
    ps.get_or_create("alerts");
    assert!(ps.topic_exists("alerts"));
    assert!(!ps.topic_exists("backups"));
}

#[test]
fn get_or_create_returns_none_once_max_topics_reached() {
    let ps = PubSub::new(10, 32, 2);
    assert!(ps.get_or_create("a").is_some());
    assert!(ps.get_or_create("b").is_some());
    assert!(ps.get_or_create("c").is_none());
    assert!(ps.get_or_create("a").is_some());
}

#[test]
fn concurrent_creation_returns_one_shared_topic() {
    let ps = Arc::new(PubSub::new(10, 32, 10));
    let barrier = Arc::new(std::sync::Barrier::new(16));
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let ps = Arc::clone(&ps);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                ps.get_or_create("alerts").unwrap()
            })
        })
        .collect();
    let topics: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert!(topics
        .iter()
        .all(|topic| Arc::ptr_eq(topic, &topics[0])));
    assert_eq!(ps.stats().topics, 1);
}
