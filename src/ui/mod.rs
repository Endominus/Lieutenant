// extern crate anyhow;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, read, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::Terminal;
use tui::widgets::{List, Block, Borders};
use anyhow::Result;
use crate::{Card, Deck};
use crate::db;

mod util;
use util::{StatefulList, MainMenuItem, Screen, DeckScreen};

struct AppState {
    mode: Screen,
    title: String,
    deck_id: i32,
    deck: Option<Deck>,
    contents: Option<Vec<Card>>,
    omnitext: String,
    dbftext: String,
    sldc: StatefulList<Card>,
    slmm: StatefulList<MainMenuItem>,
    slod: StatefulList<Deck>,
    dirty_deck: bool,
    quit: bool,
}

impl AppState {
    fn new() -> AppState {
        let mut app = AppState {
            mode: Screen::MainMenu,
            title: String::from("Main Menu"),
            deck_id: -1,
            deck: None,
            contents: None,
            sldc: StatefulList::new(),
            slmm: StatefulList::new(),
            slod: StatefulList::new(),
            quit: false,
            omnitext: String::new(),
            dbftext: String::new(),
            dirty_deck: true,
            // dsod: DeckScreen,
            // dsdb: DeckScreen,
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
                        self.switch_mode(self.slmm.get().unwrap().next);
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
                        self.deck_id = 1;
                        self.omnitext = String::new();

                        self.deck = Some(db::rdfdid(self.deck_id)?);
                        self.contents = Some(db::rvcfdid(self.deck_id)?);

                        self.mode = Screen::DeckOmni;
                        self.sldc = StatefulList::with_items(self.contents.clone().unwrap());
                        self.sldc.next();
                    }
                    _ => {}
                }
            }
            Screen::DeckOmni => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    // KeyCode::Up => { self.slod.previous(); }
                    // KeyCode::Down => { self.slod.next(); }
                    KeyCode::Enter | KeyCode::Tab => { 
                        if self.sldc.items.len() > 0 {
                            self.mode = Screen::DeckCard;
                        }
                    }
                    KeyCode::Backspace => { self.omnitext.pop(); self.usldc(); }
                    KeyCode::Char(c) => {self.omnitext.push(c); self.usldc(); }
                    _ => {}
                }
            }
            Screen::DeckCard => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    KeyCode::Up => { self.sldc.previous(); }
                    KeyCode::Down => { self.sldc.next(); }
                    KeyCode::Tab => { self.mode = Screen::DeckOmni; }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn switch_mode(&mut self, next: Option<Screen>) {
        match next {
            Some(Screen::MakeDeck) => { self.init_create_view(); }
            Some(Screen::OpenDeck) => { self.init_open_view(); }
            Some(Screen::DeckOmni) => { self.init_deck_view(); }
            Some(Screen::Settings) => { self.init_settings(); }
            Some(Screen::DeckCard) => {  }
            Some(_) => {}
            None => { self.quit = true }
        }
    }

    fn init_create_view(&mut self) {}

    fn init_deck_view(&mut self) {
        self.mode = Screen::DeckOmni;
        self.sldc = StatefulList::with_items(db::rvcfdid(1).unwrap());
        self.sldc.next();
    }
    
    fn init_settings(&mut self) {}
    
    fn init_open_view(&mut self) {
        self.mode = Screen::OpenDeck;
        let vd = db::rvd().unwrap();
        self.slod = StatefulList::with_items(vd);
        self.slod.next();
    }

    fn init_main_menu(&mut self) {
        let mut items = Vec::new();
        items.push(MainMenuItem::from_with_screen(String::from("Create a new deck"), Screen::MakeDeck));
        if false { items.push(MainMenuItem::from_with_screen(String::from("Load most recent deck"), Screen::DeckOmni)); }
        items.push(MainMenuItem::from_with_screen(String::from("Load a deck"), Screen::OpenDeck));
        items.push(MainMenuItem::from_with_screen(String::from("Settings"), Screen::Settings));
        items.push(MainMenuItem::from(String::from("Quit")));
        
        self.slmm = StatefulList::with_items(items);
        self.slmm.next();
    }

    fn usldc(&mut self) {
        // state.sldc
        // let cf = db::CardFilter::new(self.deck_id).name(Vec::from([self.omnitext.clone()]));
        // let cf = db::CardFilter::new(self.deck_id).text(self.omnitext.clone());
        let ss = self.omnitext.clone().to_lowercase();
        let cf = db::CardFilter::from(self.deck_id, & ss);
        let vcr = db::rvcfcf(cf, false);
        let vc = match vcr {
            Ok(vc) => { vc }
            _ => { Vec::new() }
        };
        self.sldc = StatefulList::with_items(vc);
        self.sldc.next();
    }
}

fn draw(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, state: &mut AppState) -> Result<()> {
    let _a = terminal.draw(|f| {

        let chunks = match state.mode {
            Screen::MainMenu | Screen::OpenDeck => {
                Layout::default().constraints([Constraint::Percentage(100)]).split(f.size())
            }
            Screen::DeckOmni | Screen::DbFilter | Screen::DbCards | Screen::DeckCard => { 
                let mut vrct = Vec::new();
                let cut = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
                    .split(f.size());
                vrct.push(cut[0]);

                vrct.append(&mut Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(26),Constraint::Min(18)].as_ref())
                    .split(cut[1]));
                vrct
            }
            Screen::Settings | Screen::MakeDeck => { Vec::new() }
        };        
            
        match state.mode {
            Screen::MainMenu => {
                let list = List::new(state.slmm.rvli())
                    .block(Block::default().title("Main Menu").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slmm.state.clone());
            }
            Screen::DbFilter => {}
            Screen::DeckOmni => {
                if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = match state.sldc.get() {
                    Some(card) => { card.ri().join("\n") }
                    None => { String::from("No cards found!") }
                };
                let mut ds = DeckScreen::new(state.omnitext.clone(), state.sldc.rvli(), text);
                
                ds.focus_omni();
                f.render_widget(ds.omni, chunks[0]);
                f.render_stateful_widget(ds.lc, chunks[1], &mut state.sldc.state.clone());
                f.render_widget(ds.fc, chunks[2]);
            }
            Screen::OpenDeck => {
                let list = List::new(state.slod.rvli())
                    .block(Block::default().title("Open Deck").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slod.state.clone());}
            Screen::Settings => {}
            Screen::MakeDeck => {}
            Screen::DbCards => {}
            Screen::DeckCard => {
                if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = state.sldc.get().unwrap().ri().join("\n");
                let mut ds = DeckScreen::new(state.omnitext.clone(), state.sldc.rvli(), text);
                
                ds.focus_lc();
                f.render_widget(ds.omni, chunks[0]);
                f.render_stateful_widget(ds.lc, chunks[1], &mut state.sldc.state.clone());
                f.render_widget(ds.fc, chunks[2]);}
        }
    })?;
    Ok(())
}

// fn main_widgets() -> Vec<Widget> {
//     let mut r = Vec::new();

//     r
// }

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