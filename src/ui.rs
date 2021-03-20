// extern crate anyhow;

// use crossterm::{input, InputEvent, KeyEvent, RawScreen, TerminalCursor};
use crate::Card;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, read, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
// use std::io;
use tui::backend::{CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::Terminal;
use tui::widgets::{List, ListItem, ListState, Block, Borders};
use anyhow::Result;

enum Screen {
    MainMenu,
    CardSearch,
    DeckView,
}

struct AppState {
    mode: Screen,
    state: ListState,
    title: String,
    deck: Option<i32>,
    contents: Option<Vec<Card>>,
    quit: bool,
}

impl AppState {
    fn new() -> AppState {
        let mut ls = ListState::default();
        ls.select(Some(0));
        AppState {
            mode: Screen::MainMenu,
            state: ls,
            title: String::from("Main Menu"),
            deck: None,
            contents: None,
            quit: false
        }
    }
}

fn draw(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, state: &mut AppState) -> Result<()> {
    let _a = terminal.draw(|mut f| {

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(5)
            .constraints(
                [
                    Constraint::Percentage(100),
                ]
                .as_ref(),
            )
            .split(f.size());

            let size = f.size();

        let items = vec![ListItem::new("Item 1"), ListItem::new("Item 2"), ListItem::new("Item 3")];
        let list = List::new(items)
            .block(Block::default().title("Main Menu").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");
        f.render_stateful_widget(list, size, &mut state.state);
    })?;
    Ok(())
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let mut state = AppState::new();

    loop {
        // terminal.hide_cursor()?;
        draw(&mut terminal, &mut state)?;

        if let Event::Key(KeyEvent { code, .. }) = read()? {
            match code {
                KeyCode::Esc => { state.quit = true; }
                KeyCode::Up => { if let Some(i) = state.state.selected() { let new = i - 1; state.state.select(Some(new)); }; }
                KeyCode::Down => { if let Some(i) = state.state.selected() { let new = i + 1; state.state.select(Some(new)); }; }
                _ => {}
            }
        }

        if state.quit {
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