//! This module contains code for the TUI that can be invoked by running
//! `music-manager edit`

use std::sync::Arc;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use eyre::Result;
use log::warn;
use tui::{backend::CrosstermBackend, Terminal};

use crate::edit::ui::check_size;

use self::{
    app::{App, AppReturn},
    inputs::events::Events,
    ui::draw,
};

pub mod app;
pub mod inputs;
pub mod io;
pub mod ui;

pub async fn start_ui(app: &Arc<tokio::sync::Mutex<App>>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    terminal.hide_cursor()?;

    let tick_rate = std::time::Duration::from_millis(200);
    let events = Events::new(tick_rate);
    {
        let mut app = app.lock().await;
        app.dispatch(io::IoEvent::Initialize).await;
    }

    if check_size(&terminal.size()?) {
        warn!("Terminal too smol");
    }

    loop {
        let mut app = app.lock().await;
        terminal.draw(|f| draw(f, &mut app).expect("Error in draw function"))?;

        // Handle inputs here
        let result = match events.next()? {
            inputs::InputEvent::Input(key) => app.do_action(key).await,
            inputs::InputEvent::Tick => app.update_on_tick().await,
        };

        if result == AppReturn::Exit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
