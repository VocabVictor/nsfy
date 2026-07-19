use super::*;

#[test]
fn separates_read_write_and_global_permissions() {
    let access = AccessControl {
        global: Some("admin".into()),
        default: DefaultPolicy::Deny,
        topics: HashMap::from([(
            "alerts".into(),
            TopicTokens {
                read: Some("reader".into()),
                write: Some("writer".into()),
            },
        )]),
        configured: true,
    };
    assert!(access.allows_topic("alerts", Permission::Read, Some("reader")));
    assert!(!access.allows_topic("alerts", Permission::Write, Some("reader")));
    assert!(access.allows_topic("alerts", Permission::Write, Some("writer")));
    assert!(access.allows_topic("other", Permission::Read, Some("admin")));
    assert!(!access.allows_topic("other", Permission::Read, Some("reader")));
}

#[test]
fn global_token_protects_topics_without_an_access_file() {
    let access = AccessControl::load(Some("admin".into()), None).unwrap();
    assert!(access.allows_topic("alerts", Permission::Read, Some("admin")));
    assert!(!access.allows_topic("alerts", Permission::Read, None));
    assert!(!access.allows_topic("alerts", Permission::Write, Some("wrong")));
}
