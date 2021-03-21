// extern crate anyhow;

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
use crate::{Card, Deck};
use crate::db;

mod util;
use util::{StatefulList, MainMenuItem, Screen};

struct AppState {
    mode: Screen,
    // state: ListState,
    title: String,
    deck: Option<i32>,
    contents: Option<Vec<Card>>,
    deck_cards: StatefulList<Card>,
    slmm: StatefulList<MainMenuItem>,
    slod: StatefulList<Deck>,
    quit: bool,
}

impl AppState {
    fn new() -> AppState {
        let mut app = AppState {
            mode: Screen::MainMenu,
            // state: ls,
            title: String::from("Main Menu"),
            deck: None,
            contents: None,
            deck_cards: StatefulList::new(),
            slmm: StatefulList::new(),
            slod: StatefulList::new(),
            quit: false
        };

        app.init_main_menu();
        app
    }

    fn handle_input(&mut self, c: KeyCode) -> Result<()> {
        match self.mode {
            Screen::MainMenu => {
                match c {
                    KeyCode::Esc => { self.quit = true; }
                    KeyCode::Up => { self.slmm.previous(); }
                    KeyCode::Down => { self.slmm.next(); }
                    KeyCode::Enter => { 
                        // let next = self.main_menu.get()?;
                        // let b = next.next;
                        self.switch_mode(self.slmm.get()?.next);
                    }
                    _ => {}
                }
            }
            Screen::OpenDeck => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    KeyCode::Up => { self.slod.previous(); }
                    KeyCode::Down => { self.slod.next(); }
                    KeyCode::Enter => { 
                        // TODO: Assign correct deck ID to config
                        self.switch_mode(Some(Screen::DeckView));
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn switch_mode(&mut self, next: Option<Screen>) {
        match next {
            Some(Screen::CreateDeck) => { self.init_create_view(); }
            Some(Screen::OpenDeck) => { self.init_open_view(); }
            Some(Screen::DeckView) => { self.init_deck_view(); }
            Some(Screen::Settings) => { self.init_settings(); }
            Some(_) => {}
            None => { self.quit = true }
        }

    }

    fn init_create_view(&mut self) {}
    fn init_deck_view(&mut self) {}
    fn init_settings(&mut self) {}
    
    fn init_open_view(&mut self) {
        self.mode = Screen::OpenDeck;
        let vd = db::rvd().unwrap();
        self.slod = StatefulList::with_items(vd);
        self.slod.next();
    }

    fn init_main_menu(&mut self) {
        let mut items = Vec::new();
        items.push(MainMenuItem::from_with_screen(String::from("Create a new deck"), Screen::CreateDeck));
        if false { items.push(MainMenuItem::from_with_screen(String::from("Load most recent deck"), Screen::DeckView)); }
        items.push(MainMenuItem::from_with_screen(String::from("Load a deck"), Screen::OpenDeck));
        items.push(MainMenuItem::from_with_screen(String::from("Settings"), Screen::Settings));
        items.push(MainMenuItem::from(String::from("Quit")));
        
        self.slmm = StatefulList::with_items(items);
        self.slmm.next();
    }
}

fn draw(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, state: &mut AppState) -> Result<()> {
    let _a = terminal.draw(|f| {
        // let chunks = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .margin(5)
        //     .constraints(
        //         [
        //             Constraint::Percentage(100),
        //         ]
        //         .as_ref(),
        //     )
        //     .split(f.size());

        let chunks = match state.mode {
            Screen::MainMenu | Screen::OpenDeck => {
                Layout::default().constraints([Constraint::Percentage(100)]).split(f.size())
            }
            Screen::DeckView | Screen::CardSearch => { Vec::new() }
            Screen::Settings | Screen::CreateDeck => { Vec::new() }
            // Screen:: => {}
            // _ => { Vec::new() }
        };
        
        if chunks.len() == 0 { println!("something went wrong"); state.quit = true; return; }
        
        // let a: Vec<ListItem> = state.slmm.items.iter().map(|mm| ListItem::new(mm.text.clone())).collect();
        // let list = List::new(a)
        //     .block(Block::default().title("Main Menu").borders(Borders::ALL))
        //     .style(Style::default().fg(Color::White))
        //     .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
        
            
        match state.mode {
            Screen::MainMenu => {
                let list = List::new(state.slmm.rvli())
                    .block(Block::default().title("Main Menu").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slmm.state.clone());
            }
            Screen::CardSearch => {}
            Screen::DeckView => {}
            Screen::OpenDeck => {
                let list = List::new(state.slod.rvli())
                    .block(Block::default().title("Open Deck").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slod.state.clone());}
            Screen::Settings => {}
            Screen::CreateDeck => {}
        }
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
            state.handle_input(code);
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