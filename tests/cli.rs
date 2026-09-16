use std::collections::BTreeMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_queries_and_non_terminal_errors_do_not_touch_reading_data() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("book-cli-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(directory.join("books")).unwrap();
    let config =
        toml::to_string(&BTreeMap::from([("library_dir", directory.join("books"))])).unwrap();
    std::fs::write(directory.join("book.toml"), &config).unwrap();
    let progress = "[positions]\nuntouched = 70\n";
    std::fs::write(directory.join("book-progress.toml"), progress).unwrap();
    for (args, status, expected) in [
        (
            vec!["--version"],
            0,
            format!("book {}", env!("CARGO_PKG_VERSION")),
        ),
        (vec!["--help"], 0, "EPUB".to_string()),
        (vec!["--unknown"], 2, "不支持的参数".to_string()),
        (vec![], 1, "请在交互式终端中运行".to_string()),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_book"))
            .args(args)
            .env("BOOK_CONFIG_DIR", &directory)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(status));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(text.contains(&expected), "{text}");
        assert_eq!(
            std::fs::read_to_string(directory.join("book.toml")).unwrap(),
            config
        );
        assert_eq!(
            std::fs::read_to_string(directory.join("book-progress.toml")).unwrap(),
            progress
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}
