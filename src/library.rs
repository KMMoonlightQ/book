use std::fs;
use std::path::{Path, PathBuf};

use crate::mobi;
use crate::text;

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

/// 一个章节：标题 + 纯文本正文 + 按当前宽度折行后的行
#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub text: String,
    pub lines: Vec<String>,
}

pub fn scan(dir: &Path) -> Vec<Book> {
    let mut books = Vec::new();
    collect(dir, &mut books);
    books.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
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
        books.push(Book { title, path, format });
    }
}

/// 加载整本书并拆分章节
pub fn load_chapters(book: &Book) -> Result<Vec<Chapter>, String> {
    match book.format {
        Format::Txt => {
            let plain = load_txt(&book.path)?;
            Ok(text::split_chapters(&plain)
                .into_iter()
                .map(|(title, body)| Chapter {
                    title,
                    text: body,
                    lines: Vec::new(),
                })
                .collect())
        }
        Format::Epub => load_epub(&book.path),
        Format::Mobi => {
            let html = mobi::load(&book.path)?;
            let plain = text::strip_html(&html);
            Ok(text::split_chapters(&plain)
                .into_iter()
                .map(|(title, body)| Chapter {
                    title,
                    text: body,
                    lines: Vec::new(),
                })
                .collect())
        }
    }
}

fn load_txt(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("读取失败: {e}"))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn load_epub(path: &Path) -> Result<Vec<Chapter>, String> {
    let mut doc = epub::doc::EpubDoc::new(path).map_err(|e| format!("打开 epub 失败: {e}"))?;
    let spine = doc.spine.clone();
    let resources = doc.resources.clone();
    let toc = doc.toc.clone();

    // spine item -> toc 标题：通过资源路径匹配目录条目
    let toc_label = |idref: &str| -> Option<String> {
        let res_path = resources.get(idref).map(|r| r.path.to_string_lossy().into_owned())?;
        toc.iter().find_map(|nav| {
            let nav_path = nav.content.to_string_lossy();
            let nav_path = nav_path.split('#').next().unwrap_or("");
            if !nav_path.is_empty()
                && (res_path.ends_with(nav_path) || nav_path.ends_with(res_path.as_str()))
            {
                Some(nav.label.trim().to_string())
            } else {
                None
            }
        })
    };

    let mut chapters = Vec::new();
    for item in &spine {
        let Some((content, _mime)) = doc.get_resource_str(&item.idref) else {
            continue;
        };
        let plain = text::strip_html(&content);
        if plain.trim().is_empty() {
            continue;
        }
        let title = toc_label(&item.idref)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| first_line_title(&plain, chapters.len() + 1));
        chapters.push(Chapter {
            title,
            text: plain,
            lines: Vec::new(),
        });
    }
    if chapters.is_empty() {
        return Err("epub 中没有可读的章节内容".to_string());
    }
    Ok(chapters)
}

fn first_line_title(text: &str, index: usize) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(24).collect())
        .unwrap_or_else(|| format!("第 {index} 节"))
}
