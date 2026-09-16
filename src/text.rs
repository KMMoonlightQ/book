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
