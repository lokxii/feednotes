use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, BufWriter},
};

use chrono::{DateTime, Local};
use ratatui::{
    self,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Padding, Paragraph},
};
use serde::{Deserialize, Serialize};
use tui_textarea::{CursorMove, Input, Key, TextArea};
use tui_widget_list::{ListBuilder, ListState, ListView};

#[derive(PartialEq, Eq)]
enum Focus {
    NewNote,
    Feed,
    Filter,
}

enum InputMode {
    Normal,
    Insert,
    View,
}

enum FeedEditingMode {
    New,
    Edit(usize),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = env!("HOME");
    let mut feed: Feed =
        match File::open(format!("{}/.local/share/feednotes/notes.json", home))
        {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader)?
            }
            Err(_) => Feed::new(),
        };
    let mut feed_view = FeedView::filter(&feed, "");

    let mut terminal = ratatui::init();
    let mut focus = Focus::Feed;
    let mut state = ListState::default();
    let mut textarea = TextArea::default();
    let mut filter = String::new();
    let mut inputmode = InputMode::Normal;
    let mut feed_editing_mode = FeedEditingMode::New;

    loop {
        terminal.draw(|f| match focus {
            Focus::Feed => {
                let [_, center_area, _] = Layout::horizontal([
                    Constraint::Min(0),
                    Constraint::Length(80),
                    Constraint::Min(0),
                ])
                .areas(f.area());

                let items = feed_view
                    .refs
                    .iter()
                    .map(|i| feed.notes[*i].clone())
                    .collect::<Vec<_>>();
                let builder = ListBuilder::new(move |context| {
                    let note = items[context.index].clone();
                    let mut item = Paragraph::new(note.text).block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(
                                note.date
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string(),
                            )
                            .padding(Padding::uniform(1)),
                    );
                    if context.is_selected {
                        item = item
                            .style(Style::default().bg(Color::Rgb(45, 50, 55)));
                    }

                    let height = item.line_count(center_area.width) as u16;
                    (item, height)
                });

                f.render_stateful_widget(
                    ListView::new(builder, feed_view.refs.len())
                        .block(Block::default())
                        .infinite_scrolling(false),
                    center_area,
                    &mut state,
                );
            }

            Focus::NewNote => {
                let area = Rect {
                    x: (f.area().width - 60) / 2,
                    y: 10,
                    width: 60,
                    height: 10,
                };

                textarea.set_block(
                    Block::bordered().border_type(BorderType::Rounded).title(
                        match inputmode {
                            InputMode::Normal => "New Note (Normal)",
                            InputMode::Insert => "New Note (Insert)",
                            InputMode::View => "New Note (View)",
                        },
                    ),
                );
                textarea.set_cursor_line_style(Style::default());
                f.render_widget(&textarea, area);
            }

            Focus::Filter => {
                let area = Rect {
                    x: (f.area().width - 60) / 2,
                    y: 10,
                    width: 60,
                    height: 3,
                };

                textarea.set_block(
                    Block::bordered().border_type(BorderType::Rounded).title(
                        match inputmode {
                            InputMode::Normal => "Filtering (Normal)",
                            InputMode::Insert => "Filtering (Insert)",
                            InputMode::View => "Filtering (View)",
                        },
                    ),
                );
                textarea.set_cursor_line_style(Style::default());
                f.render_widget(&textarea, area);
            }
        })?;

        // input
        match focus {
            Focus::Feed => {
                let Event::Key(key) = event::read()? else {
                    continue;
                };
                match key.code {
                    KeyCode::Char('q') => break,

                    KeyCode::Char('j') => state.next(),
                    KeyCode::Char('k') => state.previous(),
                    KeyCode::Char('d') => {
                        if state.selected.is_none() {
                            continue;
                        }
                        if matches!(
                            event::read()?.into(),
                            Input { key: Key::Char('d'), .. }
                        ) {
                            let i = feed_view.refs[state.selected.unwrap()];
                            feed.notes.remove(i);
                            feed_view = FeedView::filter(&feed, &filter);
                            state.previous();
                        }
                    }

                    KeyCode::Char('n') => {
                        focus = Focus::NewNote;
                        textarea = TextArea::default();
                        feed_editing_mode = FeedEditingMode::New;
                    }
                    KeyCode::Char('i') => {
                        if state.selected.is_none() {
                            continue;
                        }
                        focus = Focus::NewNote;
                        let i = feed_view.refs[state.selected.unwrap()];
                        feed_editing_mode = FeedEditingMode::Edit(i);
                        textarea = TextArea::new(
                            feed.notes[i]
                                .text
                                .lines()
                                .map(|l| l.to_string())
                                .collect(),
                        );
                    }
                    KeyCode::Char('/') => {
                        focus = Focus::Filter;
                        textarea = TextArea::new(vec![filter.clone()]);
                        textarea.move_cursor(CursorMove::End);
                        inputmode = InputMode::Insert;
                    }
                    _ => {}
                }
            }

            Focus::NewNote => {
                let event = event::read()?;
                match inputmode {
                    InputMode::Normal | InputMode::View => {
                        if matches!(
                            event.clone().into(),
                            Input { key: Key::Char('W'), .. }
                        ) && matches!(inputmode, InputMode::Normal)
                        {
                            match feed_editing_mode {
                                FeedEditingMode::New => {
                                    feed.notes.push_front(Note {
                                        text: textarea.lines().join("\n"),
                                        date: chrono::offset::Local::now(),
                                    });
                                    feed_view =
                                        FeedView::filter(&feed, &filter);
                                    focus = Focus::Feed;
                                }
                                FeedEditingMode::Edit(i) => {
                                    feed.notes[feed_view.refs[i]].text =
                                        textarea.lines().join("\n");
                                    focus = Focus::Feed;
                                }
                            }
                        } else {
                            textarea_event(
                                event,
                                &mut textarea,
                                &mut focus,
                                &mut inputmode,
                            )?
                        }
                    }
                    InputMode::Insert => match event.into() {
                        Input { key: Key::Esc, .. } => {
                            inputmode = InputMode::Normal
                        }
                        input => {
                            textarea.input(input);
                        }
                    },
                }
            }

            Focus::Filter => {
                let event = event::read()?;
                if matches!(event.clone().into(), Input { key: Key::Enter, .. })
                {
                    filter = textarea.lines().concat();
                    focus = Focus::Feed;
                    feed_view = FeedView::filter(&feed, &filter);
                    continue;
                }
                match inputmode {
                    InputMode::Insert => match event.into() {
                        Input { key: Key::Esc, .. } => {
                            inputmode = InputMode::Normal
                        }
                        input => {
                            textarea.input(input);
                        }
                    },
                    _ => textarea_event(
                        event,
                        &mut textarea,
                        &mut focus,
                        &mut inputmode,
                    )?,
                }
            }
        }
    }

    ratatui::restore();

    let feed_file =
        File::create(format!("{}/.local/share/feednotes/notes.json", home))?;
    let writer = BufWriter::new(feed_file);
    serde_json::to_writer(writer, &feed)?;
    return Ok(());
}

fn textarea_event(
    event: impl Into<Input>,
    textarea: &mut TextArea,
    focus: &mut Focus,
    inputmode: &mut InputMode,
) -> Result<(), Box<dyn std::error::Error>> {
    match event.into() {
        // normal mode
        Input { key: Key::Backspace, .. } => {
            if matches!(inputmode, InputMode::Normal) {
                *focus = Focus::Feed;
            }
        }
        Input { key: Key::Char('i'), .. } => {
            if matches!(inputmode, InputMode::Normal) {
                *inputmode = InputMode::Insert;
            }
        }
        Input { key: Key::Char('A'), .. } => {
            if matches!(inputmode, InputMode::Normal) {
                textarea.move_cursor(CursorMove::End);
                *inputmode = InputMode::Insert;
            }
        }
        Input { key: Key::Char('o'), .. } => {
            if matches!(inputmode, InputMode::Normal) {
                textarea.move_cursor(CursorMove::End);
                textarea.insert_newline();
                *inputmode = InputMode::Insert;
            }
        }
        Input { key: Key::Char('O'), .. } => {
            if matches!(inputmode, InputMode::Normal) {
                textarea.move_cursor(CursorMove::Head);
                textarea.insert_newline();
                textarea.move_cursor(CursorMove::Up);
                *inputmode = InputMode::Insert;
            }
        }
        Input { key: Key::Char('p'), .. } => {
            textarea.paste();
        }
        Input { key: Key::Char('u'), .. } => {
            textarea.undo();
        }
        Input { key: Key::Char('r'), ctrl: true, .. } => {
            textarea.redo();
        }
        Input { key: Key::Char('v'), .. } => {
            if matches!(*inputmode, InputMode::Normal) {
                textarea.start_selection();
                *inputmode = InputMode::View;
            }
        }
        Input { key: Key::Char('x'), .. } => {
            textarea.delete_next_char();
        }
        Input { key: Key::Char('>'), .. } => {
            if matches!(*inputmode, InputMode::Normal)
                && matches!(
                    event::read().unwrap().into(),
                    Input { key: Key::Char('>'), .. }
                )
            {
                let (y, x) = textarea.cursor();
                let mut lines = textarea.clone().into_lines();
                let mut new_line = String::from("    ");
                new_line += &lines[y];
                lines[y] = new_line;
                *textarea = TextArea::new(lines);
                textarea.move_cursor(CursorMove::Jump(y as u16, x as u16));
            }
        }
        Input { key: Key::Char('<'), .. } => {
            if matches!(*inputmode, InputMode::Normal)
                && matches!(
                    event::read().unwrap().into(),
                    Input { key: Key::Char('<'), .. }
                )
            {
                let (y, x) = textarea.cursor();
                let mut lines = textarea.clone().into_lines();
                let mut count = 0;
                lines[y] = lines[y]
                    .chars()
                    .skip_while(|c| {
                        count += 1;
                        *c == ' ' && count <= 4
                    })
                    .collect();
                *textarea = TextArea::new(lines);
                textarea.move_cursor(CursorMove::Jump(y as u16, x as u16));
            }
        }

        // universal movement
        Input { key: Key::Char('h'), .. } => {
            textarea.move_cursor(CursorMove::Back)
        }
        Input { key: Key::Char('j'), .. } => {
            textarea.move_cursor(CursorMove::Down)
        }
        Input { key: Key::Char('k'), .. } => {
            textarea.move_cursor(CursorMove::Up)
        }
        Input { key: Key::Char('l'), .. } => {
            textarea.move_cursor(CursorMove::Forward)
        }
        Input { key: Key::Char('w'), .. } => {
            textarea.move_cursor(CursorMove::WordForward)
        }
        Input { key: Key::Char('b'), .. } => {
            textarea.move_cursor(CursorMove::WordBack)
        }
        Input { key: Key::Char('e'), .. } => {
            textarea.move_cursor(CursorMove::WordEnd)
        }
        Input { key: Key::Char('^'), .. } => {
            textarea.move_cursor(CursorMove::Head)
        }
        Input { key: Key::Char('$'), .. } => {
            textarea.move_cursor(CursorMove::End)
        }
        Input { key: Key::Char('g'), .. } => {
            if matches!(
                event::read()?.into(),
                Input { key: Key::Char('g'), .. }
            ) {
                textarea.move_cursor(CursorMove::Top);
            }
        }
        Input { key: Key::Char('G'), .. } => {
            textarea.move_cursor(CursorMove::Bottom);
        }

        Input { key: Key::Char('d'), .. } => match *inputmode {
            InputMode::Normal => {
                let e = event::read().unwrap().into();
                match e {
                    Input { key: Key::Char('d'), .. } => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.delete_line_by_end();
                        textarea.delete_newline();
                        textarea.move_cursor(CursorMove::Down);
                    }
                    Input { key: Key::Char('w'), .. } => {
                        textarea.start_selection();
                        textarea.move_cursor(CursorMove::WordForward);
                        textarea.cut();
                        textarea.cancel_selection();
                    }
                    Input { key: Key::Char('b'), .. } => {
                        textarea.delete_word();
                    }
                    Input { key: Key::Char('i'), .. } => {
                        if matches!(
                            event::read().unwrap().into(),
                            Input { key: Key::Char('w'), .. }
                        ) {
                            textarea.move_cursor(CursorMove::WordBack);
                            textarea.delete_next_word();
                        }
                    }
                    _ => {}
                }
            }
            InputMode::View => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                *inputmode = InputMode::Normal;
            }
            InputMode::Insert => {}
        },
        Input { key: Key::Char('y'), .. } => {
            if matches!(inputmode, InputMode::View) {
                textarea.move_cursor(CursorMove::Forward);
                textarea.copy();
                textarea.cancel_selection();
                *inputmode = InputMode::Normal;
            }
        }

        Input { key: Key::Esc, .. } => {
            if matches!(inputmode, InputMode::View) {
                textarea.cancel_selection();
                *inputmode = InputMode::Normal;
            }
        }
        _ => {}
    };
    return Ok(());
}

#[derive(Clone, Serialize, Deserialize)]
struct Note {
    text: String,
    date: DateTime<Local>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Feed {
    notes: VecDeque<Note>,
}

impl Feed {
    fn new() -> Feed {
        Feed { notes: VecDeque::new() }
    }
}

#[derive(Clone)]
struct FeedView {
    refs: Vec<usize>,
}

impl FeedView {
    fn filter(feed: &Feed, pat: &str) -> Self {
        if pat == "" {
            FeedView { refs: (0..feed.notes.len()).collect() }
        } else {
            FeedView {
                refs: feed
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.text.contains(pat))
                    .map(|(i, _)| i)
                    .collect(),
            }
        }
    }
}
