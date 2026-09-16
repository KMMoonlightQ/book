//! 最小化 MOBI (PalmDB/PalmDOC) 解析器：解出正文 HTML，交给 text::strip_html 处理。

use std::fs;
use std::path::Path;

fn u16be(data: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([data[off], data[off + 1]])
}

fn u32be(data: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

pub fn load(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("读取失败: {e}"))?;
    if data.len() < 78 {
        return Err("文件太小，不是有效的 mobi".to_string());
    }

    let num_records = u16be(&data, 76) as usize;
    let records_base = 78;
    if data.len() < records_base + num_records * 8 {
        return Err("mobi 记录表不完整".to_string());
    }
    let record_offset = |i: usize| -> usize { u32be(&data, records_base + i * 8) as usize };
    let record_slice = |i: usize| -> &[u8] {
        let start = record_offset(i);
        let end = if i + 1 < num_records {
            record_offset(i + 1)
        } else {
            data.len()
        };
        &data[start.min(data.len())..end.min(data.len())]
    };

    let rec0 = record_slice(0);
    if rec0.len() < 16 {
        return Err("mobi 头记录不完整".to_string());
    }
    let compression = u16be(rec0, 0);
    let text_length = u32be(rec0, 4) as usize;
    let record_count = u16be(rec0, 8) as usize;
    let encryption = u16be(rec0, 12);
    if encryption != 0 {
        return Err("该 mobi 带有 DRM 加密，无法读取".to_string());
    }

    // MOBI 头里的文本编码（偏移 16 + 0x0C）
    let encoding = if rec0.len() >= 32 {
        u32be(rec0, 28)
    } else {
        65001
    };

    let mut raw = Vec::with_capacity(text_length);
    for i in 1..=record_count.min(num_records.saturating_sub(1)) {
        let rec = record_slice(i);
        match compression {
            1 => raw.extend_from_slice(rec),
            2 => palmdoc_decompress(rec, &mut raw),
            other => return Err(format!("不支持的 mobi 压缩格式: {other}")),
        }
    }
    raw.truncate(text_length);

    let text = if encoding == 65001 {
        String::from_utf8_lossy(&raw).into_owned()
    } else {
        decode_cp1252(&raw)
    };
    Ok(text)
}

/// PalmDOC LZ77 解压
fn palmdoc_decompress(input: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < input.len() {
        let c = input[i];
        i += 1;
        match c {
            0x01..=0x08 => {
                // 后跟 c 个字面量
                let end = (i + c as usize).min(input.len());
                out.extend_from_slice(&input[i..end]);
                i = end;
            }
            0x00 | 0x09..=0x7f => out.push(c),
            0x80..=0xbf => {
                if i >= input.len() {
                    break;
                }
                let next = input[i] as usize;
                i += 1;
                let value = ((c as usize) << 8) | next;
                let distance = (value & 0x3fff) >> 3;
                let length = (value & 0x07) + 3;
                if distance == 0 || distance > out.len() {
                    continue;
                }
                let start = out.len() - distance;
                for k in 0..length {
                    let byte = out[start + k];
                    out.push(byte);
                }
            }
            0xc0..=0xff => {
                out.push(b' ');
                out.push(c ^ 0x80);
            }
        }
    }
}

/// Windows-1252 解码（0x80-0x9F 为特殊映射，其余同 Latin-1）
fn decode_cp1252(bytes: &[u8]) -> String {
    const TABLE: [char; 32] = [
        '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}',
        '\u{8f}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}',
        '\u{2014}', '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}',
        '\u{178}',
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9f => TABLE[(b - 0x80) as usize],
            _ => b as char,
        })
        .collect()
}
