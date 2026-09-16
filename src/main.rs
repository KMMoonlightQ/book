mod config;
mod library;
mod mobi;
mod progress;
mod text;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;

use library::{Book, Chapter};
use progress::Progress;

enum Screen {
    Library,
    Reader,
    /// 章节选择页
    Toc,
}

struct ReaderState {
    book_index: usize,
    chapters: Vec<Chapter>,
    /// 当前章节
    chapter: usize,
    /// 各章节 lines 对应的折行宽度，宽度变化时重新折行
    wrap_width: u16,
    /// 当前章节内滚动到的行号（首行）
    offset: usize,
    /// 章节选择页的列表状态
    toc_state: ListState,
}

impl ReaderState {
    fn current(&self) -> &Chapter {
        &self.chapters[self.chapter]
    }

    fn max_offset(&self, page: usize) -> usize {
        self.current().lines.len().saturating_sub(page)
    }
}

struct App {
    books: Vec<Book>,
    list_state: ListState,
    screen: Screen,
    reader: Option<ReaderState>,
    progress: Progress,
}

fn main() {
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
            Screen::Toc => handle_toc_key(app, key.code),
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
    let Ok(mut chapters) = library::load_chapters(book) else {
        return;
    };
    if chapters.is_empty() {
        return;
    }
    let saved = app.progress.get(&book.key());
    let chapter = saved.chapter().min(chapters.len() - 1);
    let mut toc_state = ListState::default();
    toc_state.select(Some(chapter));
    app.reader = Some(ReaderState {
        book_index: index,
        chapters: std::mem::take(&mut chapters),
        chapter,
        wrap_width: 0,
        offset: saved.offset(),
        toc_state,
    });
    app.screen = Screen::Reader;
}

fn save_position(app: &mut App) {
    if let Some(reader) = &app.reader {
        let key = app.books[reader.book_index].key();
        app.progress.set(key, reader.chapter, reader.offset);
        app.progress.save();
    }
}

fn handle_reader_key(app: &mut App, code: KeyCode) {
    let page = reader_page_size();
    let Some(reader) = app.reader.as_mut() else {
        return;
    };
    let max_offset = reader.max_offset(page);
    match code {
        KeyCode::Esc => {
            save_position(app);
            app.screen = Screen::Library;
            app.reader = None;
        }
        KeyCode::Char('i') => {
            reader.toc_state.select(Some(reader.chapter));
            app.screen = Screen::Toc;
        }
        KeyCode::Up => {
            if reader.offset == 0 && reader.chapter > 0 {
                // 章节开头继续向上：进入上一章末尾
                reader.chapter -= 1;
                reader.offset = reader.max_offset(page);
            } else {
                reader.offset = reader.offset.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if reader.offset >= max_offset && reader.chapter + 1 < reader.chapters.len() {
                reader.chapter += 1;
                reader.offset = 0;
            } else {
                reader.offset = (reader.offset + 1).min(max_offset);
            }
        }
        KeyCode::Left | KeyCode::Char('p') => {
            if reader.offset == 0 && reader.chapter > 0 {
                reader.chapter -= 1;
                reader.offset = reader.max_offset(page);
            } else {
                reader.offset = reader.offset.saturating_sub(page);
            }
        }
        KeyCode::Right | KeyCode::Char('n') => {
            if reader.offset >= max_offset && reader.chapter + 1 < reader.chapters.len() {
                reader.chapter += 1;
                reader.offset = 0;
            } else {
                reader.offset = (reader.offset + page).min(max_offset);
            }
        }
        _ => {}
    }
}

fn handle_toc_key(app: &mut App, code: KeyCode) {
    let Some(reader) = app.reader.as_mut() else {
        return;
    };
    let len = reader.chapters.len();
    match code {
        KeyCode::Esc => app.screen = Screen::Reader,
        KeyCode::Up | KeyCode::Char('k') => {
            let i = reader.toc_state.selected().unwrap_or(0);
            reader.toc_state.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = reader.toc_state.selected().unwrap_or(0);
            reader.toc_state.select(Some((i + 1).min(len.saturating_sub(1))));
        }
        KeyCode::Enter => {
            if let Some(i) = reader.toc_state.selected() {
                reader.chapter = i;
                reader.offset = 0;
            }
            app.screen = Screen::Reader;
        }
        _ => {}
    }
}

/// 正文可见行数（底部一行留给进度条）
fn reader_page_size() -> usize {
    let Ok((_, h)) = crossterm::terminal::size() else {
        return 20;
    };
    (h as usize).saturating_sub(1).max(1)
}

fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    match app.screen {
        Screen::Library => draw_library(frame, app),
        Screen::Reader => draw_reader(frame, app),
        Screen::Toc => draw_toc(frame, app),
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

fn draw_toc(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();
    let Some(reader) = app.reader.as_mut() else {
        return;
    };

    let items: Vec<ListItem> = reader
        .chapters
        .iter()
        .map(|c| ListItem::new(Line::from(c.title.clone())))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut reader.toc_state);
}

fn draw_reader(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();
    let content_height = (area.height as usize).saturating_sub(1).max(1);

    let Some(reader) = app.reader.as_mut() else {
        return;
    };

    // 首次打开或终端宽度变化时重新折行；折行后行数可能变化，需按新旧行数比例折算进度
    if reader.wrap_width != area.width {
        let old_lines = reader.current().lines.len().max(1);
        let old_offset = reader.offset;
        for chapter in &mut reader.chapters {
            chapter.lines = text::wrap_text(&chapter.text, area.width as usize);
        }
        if reader.wrap_width != 0 {
            // 宽度变化：按比例保持阅读位置
            reader.offset = old_offset * reader.current().lines.len() / old_lines;
        }
        reader.wrap_width = area.width;
    }

    let max_offset = reader.max_offset(content_height);
    if reader.offset > max_offset {
        reader.offset = max_offset;
    }

    let visible: Vec<Line> = reader
        .current()
        .lines
        .iter()
        .skip(reader.offset)
        .take(content_height)
        .map(|l| Line::from(l.clone()))
        .collect();
    frame.render_widget(
        Paragraph::new(visible),
        ratatui::layout::Rect::new(area.x, area.y, area.width, content_height as u16),
    );

    // 底部进度条：[####------]30%，按当前章节计算
    let total = reader.current().lines.len().max(1);
    let percent = (reader.offset + content_height).min(total) * 100 / total;
    let cells = 10;
    let filled = (percent / 10 + 1).min(cells);
    let bar = format!(
        "[{}{}]{}%",
        "#".repeat(filled),
        "-".repeat(cells - filled),
        percent
    );
    frame.render_widget(
        Paragraph::new(bar)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        ratatui::layout::Rect::new(
            area.x,
            area.y + content_height as u16,
            area.width,
            1,
        ),
    );
}
