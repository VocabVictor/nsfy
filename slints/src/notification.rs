pub fn show(title: String, body: String) {
    #[cfg(windows)]
    std::thread::spawn(move || {
        use winrt_notification::{Duration, Sound, Toast};

        let _ = Toast::new(Toast::POWERSHELL_APP_ID)
            .title(&title)
            .text1(&body)
            .sound(Some(Sound::Default))
            .duration(Duration::Short)
            .show();
    });

    #[cfg(not(windows))]
    let _ = (title, body);
}
