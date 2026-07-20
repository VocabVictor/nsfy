use super::*;

#[test]
fn advanced_preferences_have_safe_defaults() {
    let config = config::StoredConfig::default();
    let preferences = DesktopPreferences::from_config(&config);
    assert!(!preferences.auto_start);
    assert!(preferences.start_minimized);
    assert_eq!(preferences.dnd_shortcut, "Ctrl+Alt+D");
}

#[test]
fn configured_shortcuts_are_parsed() {
    let mut config = config::StoredConfig::default();
    config.advanced = serde_json::json!({
        "autoStart": true,
        "startMinimized": false,
        "dndShortcut": "Ctrl+Shift+M",
        "showShortcut": "Ctrl+Shift+N"
    });
    let preferences = DesktopPreferences::from_config(&config);
    assert!(preferences.auto_start);
    assert!(!preferences.start_minimized);
    assert!(parse_shortcut(&preferences.dnd_shortcut).is_ok());
}
