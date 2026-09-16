mod config;
mod library;
mod mobi;
mod progress;
mod text;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;
use std::io::IsTerminal;

use library::{Book, Format};
use progress::Progress;

enum Screen {
    Library,
    Reader,
}

struct ReaderState {
    book_index: usize,
    /// 清洗后的纯文本（用于宽度变化时重新折行）
    raw: String,
    /// 折行后的全部行
    lines: Vec<String>,
    /// lines 对应的折行宽度，宽度变化时重新折行
    wrap_width: u16,
    /// 当前滚动到的行号（首行）
    offset: usize,
}

struct App {
    books: Vec<Book>,
    list_state: ListState,
    screen: Screen,
    reader: Option<ReaderState>,
    progress: Progress,
}

fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if !arguments.is_empty() {
        if arguments.len() == 1 {
            if arguments[0] == "--version" || arguments[0] == "-V" {
                println!("book {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            if arguments[0] == "--help" || arguments[0] == "-h" {
                println!(
                    "book — 终端电子书阅读器\n\n用法: book [--help | --version]\n\n\
                     支持 TXT / EPUB / MOBI。首次运行会创建 ~/.config/book.toml。\n\
                     默认书库: ~/books；可通过 library_dir 修改。\n\
                     书架: ↑/↓ 或 j/k 选择，Enter 打开，q 退出。\n\
                     阅读: ↑/↓ 滚动，←/→ 或 p/n 翻页，Esc 保存进度并返回书架。\n\
                     BOOK_CONFIG_DIR 可指定独立的配置和阅读进度目录。"
                );
                return;
            }
        }
        eprintln!("不支持的参数；请运行 book --help 查看用法。");
        std::process::exit(2);
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("请在交互式终端中运行 book。");
        std::process::exit(1);
    }

    let (_, library_dir) = match config::load() {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };

    let mut app = App {
        books: library::scan(&library_dir),
        list_state: ListState::default(),
        screen: Screen::Library,
        reader: None,
        progress: Progress::load(),
    };
    if !app.books.is_empty() {
        app.list_state.select(Some(0));
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    app.progress.save();
    if let Err(e) = result {
        eprintln!("运行出错: {e}");
        std::process::exit(1);
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }

        match app.screen {
            Screen::Library => {
                if handle_library_key(app, key.code) {
                    return Ok(());
                }
            }
            Screen::Reader => handle_reader_key(app, key.code),
        }
    }
}

/// 返回 true 表示退出程序
fn handle_library_key(app: &mut App, code: KeyCode) -> bool {
    let len = app.books.len();
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Up | KeyCode::Char('k') if len > 0 => {
            let i = app.list_state.selected().unwrap_or(0);
            app.list_state.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') if len > 0 => {
            let i = app.list_state.selected().unwrap_or(0);
            app.list_state.select(Some((i + 1).min(len - 1)));
        }
        KeyCode::Enter if len > 0 => {
            if let Some(i) = app.list_state.selected() {
                open_book(app, i);
            }
        }
        _ => {}
    }
    false
}

fn open_book(app: &mut App, index: usize) {
    let book = &app.books[index];
    if let Ok(raw) = library::load_text(book) {
        let plain = if matches!(book.format, Format::Txt) {
            raw
        } else {
            text::strip_html(&raw)
        };
        let saved = app.progress.get(&book.key());
        app.reader = Some(ReaderState {
            book_index: index,
            raw: plain,
            lines: Vec::new(),
            wrap_width: 0,
            offset: saved,
        });
        app.screen = Screen::Reader;
    }
}

fn handle_reader_key(app: &mut App, code: KeyCode) {
    let page = reader_page_size();
    let Some(reader) = app.reader.as_mut() else {
        return;
    };
    let max_offset = reader.lines.len().saturating_sub(page);
    match code {
        KeyCode::Esc => {
            // 保存进度并返回书架
            let key = app.books[reader.book_index].key();
            app.progress.set(key, reader.offset);
            app.progress.save();
            app.screen = Screen::Library;
            app.reader = None;
        }
        KeyCode::Up => reader.offset = reader.offset.saturating_sub(1),
        KeyCode::Down => reader.offset = (reader.offset + 1).min(max_offset),
        KeyCode::Left | KeyCode::Char('p') => reader.offset = reader.offset.saturating_sub(page),
        KeyCode::Right | KeyCode::Char('n') => {
            reader.offset = (reader.offset + page).min(max_offset)
        }
        _ => {}
    }
}

/// 正文可见行数
fn reader_page_size() -> usize {
    let Ok((_, h)) = crossterm::terminal::size() else {
        return 20;
    };
    (h as usize).max(1)
}

fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    match app.screen {
        Screen::Library => draw_library(frame, app),
        Screen::Reader => draw_reader(frame, app),
    }
}

fn draw_library(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();

    let items: Vec<ListItem> = app
        .books
        .iter()
        .map(|b| ListItem::new(Line::from(b.title.clone())))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_reader(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();

    let content_width = area.width;
    let content_height = area.height as usize;

    let Some(reader) = app.reader.as_mut() else {
        return;
    };

    // 首次打开或终端宽度变化时重新折行；折行后行数可能变化，需按新旧行数比例折算进度
    if reader.wrap_width != content_width {
        let old_lines = reader.lines.len().max(1);
        let old_offset = reader.offset;
        reader.lines = text::wrap_text(&reader.raw, content_width as usize);
        if reader.wrap_width != 0 {
            // 宽度变化：按比例保持阅读位置
            reader.offset = old_offset * reader.lines.len() / old_lines;
        }
        reader.wrap_width = content_width;
    }

    let page = content_height.max(1);
    let max_offset = reader.lines.len().saturating_sub(page);
    if reader.offset > max_offset {
        reader.offset = max_offset;
    }

    let visible: Vec<Line> = reader
        .lines
        .iter()
        .skip(reader.offset)
        .take(page)
        .map(|l| Line::from(l.clone()))
        .collect();
    frame.render_widget(Paragraph::new(visible), area);
}
