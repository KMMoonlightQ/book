//! HTML 清洗与按显示宽度折行。

use unicode_width::UnicodeWidthChar;

/// 把 HTML（epub 章节 / mobi 正文）转成适合终端展示的纯文本。
pub fn strip_html(html: &str) -> String {
    const BLOCK_TAGS: [&str; 13] = [
        "p", "div", "br", "h1", "h2", "h3", "h4", "h5", "h6", "li", "tr", "table", "section",
    ];

    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                // 读取标签名
                let mut name = String::new();
                let mut closing = false;
                if matches!(chars.peek(), Some('/')) {
                    closing = true;
                    chars.next();
                }
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() {
                        name.push(nc.to_ascii_lowercase());
                        chars.next();
                    } else {
                        break;
                    }
                }
                let skip_content = name == "script" || name == "style";
                // 吞掉标签剩余部分
                for nc in chars.by_ref() {
                    if nc == '>' {
                        break;
                    }
                }
                if skip_content && !closing {
                    // 跳过 script/style 内容直到对应闭合标签
                    let end = format!("</{}", name);
                    let mut window = String::new();
                    for nc in chars.by_ref() {
                        window.push(nc);
                        if window.to_ascii_lowercase().ends_with(&end) {
                            break;
                        }
                    }
                } else if BLOCK_TAGS.contains(&name.as_str()) {
                    out.push('\n');
                }
            }
            '&' => {
                let mut entity = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == ';' {
                        chars.next();
                        break;
                    }
                    if !nc.is_ascii_alphanumeric() && nc != '#' {
                        break;
                    }
                    entity.push(nc);
                    chars.next();
                    if entity.len() > 10 {
                        break;
                    }
                }
                if entity.is_empty() {
                    out.push('&');
                } else {
                    out.push_str(&decode_entity(&entity));
                }
            }
            _ => out.push(c),
        }
    }

    // 折叠多余空白
    let mut result = String::with_capacity(out.len());
    let mut blank_run = 0;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                result.push('\n');
            }
        } else {
            blank_run = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    while result.starts_with('\n') {
        result.remove(0);
    }
    result
}

fn decode_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        "nbsp" => " ".into(),
        "mdash" => "—".into(),
        "ndash" => "–".into(),
        "hellip" => "…".into(),
        "ldquo" => "“".into(),
        "rdquo" => "”".into(),
        "lsquo" => "‘".into(),
        "rsquo" => "’".into(),
        _ => {
            if let Some(num) = entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse::<u32>().ok()))
            {
                char::from_u32(num)
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
    }
}

/// 按终端显示宽度折行（兼容中文等宽字符），返回行列表。
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let raw_line = raw_line.trim_end();
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut current_width = 0;
        for ch in raw_line.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + w > width && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            current.push(ch);
            current_width += w;
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// 判断一行是否为章节标题（第X章/回/节/卷、Chapter N、楔子/序章/尾声/番外等）
pub fn is_chapter_heading(line: &str) -> bool {
    let s = line.trim();
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 2 || chars.len() > 40 {
        return false;
    }
    if chars[0] == '第' {
        let mut i = 1;
        while i < chars.len()
            && (chars[i].is_ascii_digit() || "零一二三四五六七八九十百千万两".contains(chars[i]))
        {
            i += 1;
        }
        if i > 1 && i < chars.len() && "章节回卷部幕".contains(chars[i]) {
            // “第X章”后应是空白或行尾，避免误匹配正文（如“第1章第1段……”）
            return i + 1 == chars.len() || chars[i + 1].is_whitespace();
        }
    }
    if s.to_ascii_uppercase().starts_with("CHAPTER ") {
        return true;
    }
    for prefix in ["楔子", "序章", "序言", "尾声", "终章", "番外"] {
        if s.starts_with(prefix) {
            return true;
        }
    }
    false
}

/// 把纯文本按章节标题拆分为 (标题, 正文) 列表；开头没有标题的部分也保留为一章
pub fn split_chapters(text: &str) -> Vec<(String, String)> {
    let mut chapters: Vec<(String, String)> = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if is_chapter_heading(line) {
            if !current.trim().is_empty() {
                push_chapter(&mut chapters, std::mem::take(&mut current));
            }
            current.clear();
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() || chapters.is_empty() {
        push_chapter(&mut chapters, current);
    }
    chapters
}

fn push_chapter(chapters: &mut Vec<(String, String)>, body: String) {
    let title = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(24).collect())
        .unwrap_or_else(|| format!("第 {} 节", chapters.len() + 1));
    chapters.push((title, body));
}
