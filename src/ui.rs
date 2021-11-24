use std::{
    io::{stdout, Stdout},
    thread,
    time::{Duration, Instant},
};

use crossbeam::channel::{self, Receiver, Sender};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, List, Paragraph},
    widgets::{Borders, ListItem},
    Frame, Terminal,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use eyre::Result;

use unicode_width::UnicodeWidthStr;

use crate::util::{EditApp, InputMode};

#[derive(Debug)]
pub enum Event<I> {
    Input(I),
    DoneEditing,
    Tick,
}
pub fn init_tui() -> Result<(
    Terminal<CrosstermBackend<std::io::Stdout>>,
    Sender<Event<KeyEvent>>,
    Receiver<Event<KeyEvent>>,
)> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    // Input handling
    let (tx, rx) = channel::unbounded();
    let (tx1, _rx1) = (tx.clone(), rx.clone());

    let tick_rate = Duration::from_millis(200);
    // The thread will run detached, and will quit when our app quits
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events send tick event
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            // Crossterm events
            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx1.send(Event::Input(key)).unwrap();
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx1.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });
    terminal.clear()?;
    Ok((terminal, tx, rx))
}

pub fn exit_tui(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

pub fn draw_edit<B: Backend>(f: &mut Frame<B>, app: &mut EditApp) -> Result<()> {
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(f.size());

    let input_struct = &app.input;
    let input_text = input_struct.buffer.lock().unwrap().clone();
    let input_bool = {
        input_struct
            .editing
            .load(std::sync::atomic::Ordering::Relaxed)
    };

    let title = Paragraph::new(Span::styled(
        "music-manager: Edit songs",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL))
    .alignment(tui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);

    let middle_chunks = Layout::default()
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .direction(Direction::Horizontal)
        .horizontal_margin(0)
        .split(chunks[1]);

    // Directory view (middle_chunks[0])
    let dir_list = List::new(
        app.dir_list
            .items
            .iter()
            .map(|e| ListItem::new(e.as_str()))
            .collect::<Vec<ListItem>>(),
    )
    .block(Block::default().borders(Borders::ALL))
    .highlight_style(
        Style::default()
            .add_modifier(Modifier::ITALIC)
            .fg(Color::Yellow),
    )
    .highlight_symbol(">>");
    f.render_stateful_widget(dir_list, middle_chunks[0], &mut app.dir_list.state);

    // Song metadata view (middle_chunks[1])
    let song_metadata_list = List::new(
        app.current_editing_song
            .items
            .iter()
            .map(|e| ListItem::new(e.as_str()))
            .collect::<Vec<ListItem>>(),
    )
    .block(Block::default().borders(Borders::ALL))
    .highlight_style(Style::default().fg(Color::Blue))
    .highlight_symbol(">>");

    f.render_stateful_widget(
        song_metadata_list,
        middle_chunks[1],
        &mut app.current_editing_song.state,
    );

    // Input bar
    let input = Paragraph::new(input_text)
        .style(match input_bool {
            false => Style::default(),
            true => Style::default().fg(Color::Yellow),
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(match input_bool {
                    false => "Input bar",
                    true => "<ESC> to exit without committing, <Enter> to commit",
                })
                .title_alignment(tui::layout::Alignment::Center),
        );

    f.render_widget(input, chunks[2]);
    Ok(())
}
