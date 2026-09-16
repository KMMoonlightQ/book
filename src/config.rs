use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const DEFAULT_CONFIG: &str = r#"# 电子书阅读器配置
# 书籍源文件夹，支持 txt / epub / mobi
library_dir = "~/books"
"#;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub library_dir: String,
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home_dir().join(rest)
    } else if path == "~" {
        home_dir()
    } else {
        PathBuf::from(path)
    }
}

fn config_dir() -> PathBuf {
    std::env::var_os("BOOK_CONFIG_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn config_path() -> PathBuf {
    config_dir().join("book.toml")
}

pub fn progress_path() -> PathBuf {
    config_dir().join("book-progress.toml")
}

/// 读取配置；配置文件不存在时写入默认模板并创建默认书库目录。
pub fn load() -> Result<(Config, PathBuf), String> {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("无法创建配置目录: {e}"))?;
        }
        fs::write(&path, DEFAULT_CONFIG).map_err(|e| format!("无法写入默认配置: {e}"))?;
        let dir = home_dir().join("books");
        let _ = fs::create_dir_all(&dir);
        return Err(format!(
            "已创建默认配置文件 {}\n请把书籍放入 ~/books（或修改 library_dir 指向你的书籍目录），然后重新启动。",
            path.display()
        ));
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("无法读取 {}: {e}", path.display()))?;
    let config: Config =
        toml::from_str(&raw).map_err(|e| format!("配置文件 {} 解析失败: {e}", path.display()))?;
    let dir = expand_tilde(&config.library_dir);
    if !dir.is_dir() {
        return Err(format!(
            "书籍目录 {} 不存在，请修改 {} 中的 library_dir",
            dir.display(),
            path.display()
        ));
    }
    Ok((config, dir))
}
