use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use eyre::Result;
use tui_logger::TuiLoggerSmartWidget;

use unicode_width::UnicodeWidthStr;

use super::app::{actions::Actions, App, AppActiveWidgetState};

pub fn draw<B>(f: &mut Frame<B>, app: &mut App) -> Result<()>
where
    B: Backend,
{
    // TODO check for valid size
    let selected_style = Style::default().fg(Color::Yellow);
    let default_style = Style::default().fg(Color::White);

    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(0),
                Constraint::Length(6),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(f.size());
    //
    // Title
    //
    let title = Paragraph::new(Span::styled(
        "music-manager: Edit songs",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::NONE))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    //
    // Input Bar
    //
    let input_bar = Paragraph::new(app.input_buffer.get_buffer())
        .style(Style::default())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(match app.is_selected(AppActiveWidgetState::InputBar) {
                    true => selected_style,
                    false => default_style,
                })
                .title("Input Bar"),
        );
    f.render_widget(input_bar, chunks[1]);
    if app.is_input {
        f.set_cursor(
            chunks[1].x + app.input_buffer.get_buffer().width() as u16 + 1,
            chunks[1].y + 1,
        );
    } else {
        // Hide the cursor, except we don't have to do anything
        // f.draw automagically hides the cursor
    }

    // Log display
    let log_display = TuiLoggerSmartWidget::default()
        .border_style(
            match app.is_selected(super::app::AppActiveWidgetState::LogViewer) {
                true => selected_style,
                false => default_style,
            },
        )
        .style_info(Style::default().fg(Color::Green))
        .style_debug(Style::default().fg(Color::Cyan))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_error(Style::default().fg(Color::Red))
        .style_trace(Style::default().fg(Color::DarkGray))
        .state(&app.logs_state);
    f.render_widget(log_display, chunks[2]);

    //
    //
    // Middle section
    //
    //

    let middle_chunks = Layout::default()
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .direction(Direction::Horizontal)
        .horizontal_margin(0)
        .split(chunks[3]);

    let dir_list = List::new(
        app.dirlist
            .current_dir_file_names
            .iter()
            .map(|e| ListItem::new(e.as_str()))
            .collect::<Vec<ListItem>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(
                match app.is_selected(super::app::AppActiveWidgetState::DirListing) {
                    true => selected_style,
                    false => default_style,
                },
            ),
    )
    .style(default_style)
    .highlight_style(selected_style.add_modifier(Modifier::ITALIC))
    .highlight_symbol(">>");
    f.render_stateful_widget(dir_list, middle_chunks[0], &mut app.dirlist.state);

    //
    // Song list
    //
    let song_metadata_list = List::new(
        app.current_selected_song
            .items
            .iter()
            .map(|e| ListItem::new(e.as_str()))
            .collect::<Vec<ListItem>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(
                match app.is_selected(super::app::AppActiveWidgetState::MetadataEditor) {
                    true => selected_style,
                    false => default_style,
                },
            ),
    )
    .style(default_style)
    .highlight_style(Style::default().fg(Color::Blue))
    .highlight_symbol(">>");
    f.render_stateful_widget(
        song_metadata_list,
        middle_chunks[1],
        &mut app.current_selected_song.state,
    );

    let help = draw_help(app.get_actions());
    f.render_widget(help, chunks[4]);

    Ok(())
}

fn draw_help(actions: &Actions) -> Table {
    let key_style = Style::default().fg(Color::LightCyan);
    let help_style = Style::default().fg(Color::Gray);

    let mut rows = vec![];
    for action in actions.actions() {
        let mut first = true;
        for key in action.keys() {
            let help = if first {
                first = false;
                action.to_string()
            } else {
                String::from("")
            };
            let row = Row::new(vec![
                Cell::from(Span::styled(key.to_string(), key_style)),
                Cell::from(Span::styled(help, help_style)),
            ]);
            rows.push(row);
        }
    }

    // TODO Make it show all actions
    Table::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Help"),
        )
        .widths(&[Constraint::Length(11), Constraint::Min(20)])
        .column_spacing(1)
}


pub fn check_size(rect: &Rect) -> bool {
    !(rect.width < 30 || rect.height < 15)
}

#[cfg(test)]
mod tests {
    use tui::layout::Rect;

    use crate::edit::ui::check_size;

    #[test]
    fn should_warn_on_small_terminal() {
        let rect = Rect {
            x: 1,
            y: 1,
            width: 15,
            height: 19,
        };

        assert!(!check_size(&rect));
    }
}
