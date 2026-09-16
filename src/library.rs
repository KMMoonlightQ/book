use std::fs;
use std::path::{Path, PathBuf};

use crate::mobi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Txt,
    Epub,
    Mobi,
}

#[derive(Debug, Clone)]
pub struct Book {
    pub title: String,
    pub path: PathBuf,
    pub format: Format,
}

impl Book {
    /// 用作进度记录的稳定 key
    pub fn key(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

pub fn scan(dir: &Path) -> Vec<Book> {
    let mut books = Vec::new();
    collect(dir, &mut books);
    books.sort_by_key(|book| book.title.to_lowercase());
    books
}

fn collect(dir: &Path, books: &mut Vec<Book>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, books);
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let format = match ext.to_ascii_lowercase().as_str() {
            "txt" => Format::Txt,
            "epub" => Format::Epub,
            "mobi" => Format::Mobi,
            _ => continue,
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未命名")
            .to_string();
        books.push(Book {
            title,
            path,
            format,
        });
    }
}

pub fn load_text(book: &Book) -> Result<String, String> {
    match book.format {
        Format::Txt => load_txt(&book.path),
        Format::Epub => load_epub(&book.path),
        Format::Mobi => mobi::load(&book.path),
    }
}

fn load_txt(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("读取失败: {e}"))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn load_epub(path: &Path) -> Result<String, String> {
    let mut doc = epub::doc::EpubDoc::new(path).map_err(|e| format!("打开 epub 失败: {e}"))?;
    let spine = doc.spine.clone();
    let mut html = String::new();
    for item in &spine {
        if let Some((content, _mime)) = doc.get_resource_str(&item.idref) {
            html.push_str(&content);
            html.push('\n');
        }
    }
    if html.is_empty() {
        return Err("epub 中没有可读的章节内容".to_string());
    }
    Ok(html)
}
