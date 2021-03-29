// extern crate anyhow;

use tui::widgets::{Clear, ListItem, Paragraph};
use tui::layout::Rect;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, read, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rusqlite::Connection;
use std::{convert::TryInto, io::stdout};
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::Terminal;
use tui::widgets::{List, Block, Borders};
use anyhow::Result;
use crate::{Card, Deck, NewCard};
use crate::db;
// use crate::db::DbContext;

mod util;
use util::{StatefulList, MainMenuItem, Screen, DeckScreen, MakeDeckScreen, MakeDeckContents, Omnitext};

use self::util::MakeDeckFocus;

struct AppState {
    mode: Screen,
    mode_p: Screen,
    title: String,
    deck_id: i32,
    deck: Option<Deck>,
    contents: Option<Vec<NewCard>>,
    omnitext: Omnitext,
    dbftext: Omnitext,
    sldc: StatefulList<NewCard>,
    slmm: StatefulList<MainMenuItem>,
    slod: StatefulList<Deck>,
    sldbc: StatefulList<NewCard>,
    mdc: MakeDeckContents,
    dirty_deck: bool,
    dirty_cards: Vec<NewCard>,
    dbc: Connection,
    quit: bool,
}


impl AppState {
    fn new() -> AppState {
        let conn = Connection::open("lieutenant.db").unwrap();
        db::add_regexp_function(&conn).unwrap();
        let mut app = AppState {
            mode: Screen::MainMenu,
            mode_p: Screen::MainMenu,
            title: String::from("Main Menu"),
            deck_id: -1,
            deck: None,
            contents: None,
            sldc: StatefulList::new(),
            slmm: StatefulList::new(),
            slod: StatefulList::new(),
            sldbc: StatefulList::new(),
            mdc: MakeDeckContents::default(),
            quit: false,
            omnitext: Omnitext::default(),
            dbftext: Omnitext::default(),
            dirty_deck: true,
            dirty_cards: Vec::new(),
            dbc: conn,
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
                        self.deck_id = self.slod.get().unwrap().id;
                        self.init_deck_view();
                    }
                    _ => {}
                }
            }
            Screen::DeckOmni => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    // KeyCode::Up => { self.slod.previous(); }
                    // KeyCode::Down => { self.slod.next(); }
                    KeyCode::Left => { self.omnitext.left(); }
                    KeyCode::Right => { self.omnitext.right(); }
                    KeyCode::Enter => { 
                        if self.sldc.items.len() > 0 {
                            self.mode = Screen::DeckCard;
                        }
                    }
                    KeyCode::Tab => {
                        self.mode = Screen::DbFilter;
                    }
                    KeyCode::Backspace => { self.omnitext.backspace(); self.uslvc(); }
                    KeyCode::Delete => { self.omnitext.delete(); self.uslvc(); }
                    KeyCode::Char(c) => {self.omnitext.insert(c); self.uslvc(); }
                    _ => {}
                }
            }
            Screen::DeckCard => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    KeyCode::Up => { self.sldc.previous(); }
                    KeyCode::Down => { self.sldc.next(); }
                    KeyCode::Tab => { self.mode = Screen::DeckOmni; }
                    KeyCode::Backspace | KeyCode::Delete => {
                        db::dcntodc(&self.dbc, self.sldc.get().unwrap().name.clone(), self.deck_id);
                        self.sldc.remove();
                        self.dirty_deck = true;
                    }
                    _ => {}
                }
            }

            Screen::DbFilter => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    // KeyCode::Up => { self.slod.previous(); }
                    // KeyCode::Down => { self.slod.next(); }
                    KeyCode::Left => { self.dbftext.left(); }
                    KeyCode::Right => { self.dbftext.right(); }
                    KeyCode::Enter => { 
                        self.uslvc();
                        if self.sldbc.items.len() > 0 {
                            self.mode = Screen::DbCards;
                        }
                    }
                    KeyCode::Tab => {
                        // self.mode = Screen::DeckOmni;
                        self.switch_mode(Some(Screen::DeckOmni));
                    }
                    KeyCode::Backspace => { self.dbftext.backspace(); }
                    KeyCode::Delete => { self.dbftext.delete(); }
                    KeyCode::Char(c) => {self.dbftext.insert(c); }
                    _ => {}
                }
            }
            Screen::DbCards => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    KeyCode::Up => { self.sldbc.previous(); }
                    KeyCode::Down => { self.sldbc.next(); }
                    KeyCode::Tab => { self.mode = Screen::DbFilter; }
                    KeyCode::Enter => {
                        let a = self.sldbc.get().unwrap();
                        db::icntodc(&self.dbc, a.name.clone(), self.deck_id.try_into().unwrap()).unwrap();
                        self.dirty_deck = true;
                        self.dirty_cards.push(db::rcfn(&self.dbc, a.name.clone())?);
                        // let mut a = &self.contents.unwrap();
                    }
                    _ => {}
                }
            }
            Screen::MakeDeck => {
                match c {
                    KeyCode::Esc => { self.mode = Screen::MainMenu; }
                    KeyCode::Enter => {
                        match self.mdc.focus {
                            MakeDeckFocus::Title => { self.mdc.focus = MakeDeckFocus::Commander; }
                            MakeDeckFocus::Commander => { 
                                match db::rcfn(&self.dbc, self.mdc.commander.clone()) {
                                    Ok(_) => {
                                        self.deck_id = db::ideck(
                                            &self.dbc, 
                                            self.mdc.title.clone(), 
                                            self.mdc.commander.clone(), 
                                            "Commander").unwrap();
                                        self.mdc = MakeDeckContents::default();
                                        self.init_deck_view();
                                    }
                                    Err(_) => {
                                        self.mode_p = self.mode;
                                        self.mode = Screen::Error("Commander name not found in database.\nPlease check spelling and try again.\nPress Enter to continue.");
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => { 
                        match self.mdc.focus {
                            MakeDeckFocus::Title => { self.mdc.title.pop(); }
                            MakeDeckFocus::Commander => { self.mdc.commander.pop(); }
                        }
                    }
                    KeyCode::Char(ch) => {
                        match self.mdc.focus {
                            MakeDeckFocus::Title => { self.mdc.title.push(ch); }
                            MakeDeckFocus::Commander => { self.mdc.commander.push(ch); }
                        }
                    }
                    _ => {}
                }
            }
            Screen::Error(_) => {
                if KeyCode::Enter == c {
                    self.mode = self.mode_p;
                }
            }
            _ => {}
        }

        Ok(())
    }

    // fn reset(&mut self) {
    //     self.deck_id = -1;
    //     self.omnitext = String::new();
    //     self.dbftext = String::new();
    //     self.contents = None;
    // }

    fn switch_mode(&mut self, next: Option<Screen>) {
        match next {
            Some(Screen::MakeDeck) => { self.mode = Screen::MakeDeck; }
            Some(Screen::OpenDeck) => { self.init_open_view(); }
            Some(Screen::DeckOmni) => { /* Set deck_id here */ self.init_deck_view(); }
            Some(Screen::Settings) => { self.init_settings(); }
            Some(Screen::DeckCard) => {  }
            Some(_) => {}
            None => { self.quit = true }
        }
    }

    fn init_create_view(&mut self) {}

    fn init_deck_view(&mut self) {

        // self.deck_id = 1;
        // self.omnitext = String::new();
        
        self.deck = Some(db::rdfdid(&self.dbc, self.deck_id).unwrap());
        if self.dirty_deck {
            self.contents = Some(db::rvcfdid(&self.dbc, self.deck_id).unwrap());
        }
        
        self.dirty_deck = false;
        self.mode = Screen::DeckOmni;
        self.sldc = StatefulList::with_items(self.contents.clone().unwrap());
        self.sldc.next();
    }
    
    fn init_settings(&mut self) {}
    
    fn init_open_view(&mut self) {
        self.mode = Screen::OpenDeck;
        let vd = db::rvd(&self.dbc).unwrap();
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

    fn uslvc(&mut self) {
        // state.sldc
        // let cf = db::CardFilter::new(self.deck_id).name(Vec::from([self.omnitext.clone()]));
        // let cf = db::CardFilter::new(self.deck_id).text(self.omnitext.clone());
        let (ss, general, target) = match self.mode {
            Screen::DeckOmni => { (
                self.omnitext.get(),
                false,
                &mut self.sldc
            )}
            Screen::DbFilter => { (
                self.dbftext.get(),
                true,
                &mut self.sldbc
            )}
            _ => { panic!(); }
        };
        let cf = db::CardFilter::from(self.deck_id, & ss);
        let vcr = db::rvcfcf(&self.dbc, cf, general);
        let vc = match vcr {
            Ok(vc) => { vc }
            _ => { Vec::new() }
        };
        *target = StatefulList::with_items(vc);
        target.next();
    }

    pub fn rvli(&self) -> Vec<ListItem> {
        match self.mode {
            Screen::DbCards | Screen::DbFilter => { self.sldbc.rvli() }
            Screen::DeckCard | Screen::DeckOmni => { self.sldc.rvli() }
            Screen::MainMenu => { self.slmm.rvli() }
            Screen::OpenDeck => { self.slod.rvli() }
            _ => { Vec::new() }
        }
    }

    pub fn rvlis(&self) -> Vec<ListItem> {
        let mut a = self.contents.clone().unwrap();
        a.append(&mut self.dirty_cards.clone());

        self.sldbc.rvlis(a)
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
            Screen::MakeDeck => { 
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Length(3)])
                    .split(f.size())
            }
            Screen::Error(_) => { 
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(f.size())
            }
            Screen::Settings => { Vec::new() }
        };        
            
        match state.mode {
            Screen::MainMenu => {
                let list = List::new(state.rvli())
                    .block(Block::default().title("Main Menu").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slmm.state.clone());
            }
            Screen::DbFilter => {
                // if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = match state.sldbc.get() {
                    Some(card) => { card.ri().join("\n") }
                    None => { String::from("No cards found!") }
                };
                let mut ds = DeckScreen::new(
                    state.dbftext.get_styled(), 
                    state.rvlis(), 
                    text, 
                    state.mode);
                
                ds.focus_omni(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_stateful_widget(ds.lc, chunks[1], &mut state.sldbc.state.clone());
                f.render_widget(ds.fc, chunks[2]);
            }
            Screen::DeckOmni => {
                // if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = match state.sldc.get() {
                    Some(card) => { card.ri().join("\n") }
                    None => { String::from("No cards found!") }
                };
                let mut ds = DeckScreen::new(state.omnitext.get_styled(), state.rvli(), text, state.mode);
                
                ds.focus_omni(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_stateful_widget(ds.lc, chunks[1], &mut state.sldc.state.clone());
                f.render_widget(ds.fc, chunks[2]);
            }
            Screen::OpenDeck => {
                let list = List::new(state.rvli())
                    .block(Block::default().title("Open Deck").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slod.state.clone());}
            Screen::Settings => {}
            Screen::Error(s) => {
                let err_message = Paragraph::new(s)
                    .block(Block::default().title("Error!"));
                let area = centered_rect(60, 20, f.size());
                f.render_widget(Clear, area);
                f.render_widget(err_message, area);
            }
            Screen::MakeDeck => {
                let mds = MakeDeckScreen::new(&state.mdc);
                f.render_widget(mds.title_entry, chunks[0]);
                f.render_widget(mds.commander_entry, chunks[1]);
            }
            Screen::DbCards => {
                let text = state.sldbc.get().unwrap().ri().join("\n");
                let mut ds = DeckScreen::new(
                    state.dbftext.get_styled(), 
                    state.rvlis(), 
                    text, 
                    state.mode);
                
                ds.focus_lc(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_stateful_widget(ds.lc, chunks[1], &mut state.sldbc.state.clone());
                f.render_widget(ds.fc, chunks[2]);
            }
            Screen::DeckCard => {
                // if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = state.sldc.get().unwrap().ri().join("\n");
                let mut ds = DeckScreen::new(state.omnitext.get_styled(), state.rvli(), text, state.mode);
                
                ds.focus_lc(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_stateful_widget(ds.lc, chunks[1], &mut state.sldc.state.clone());
                f.render_widget(ds.fc, chunks[2]);}
        }
    })?;
    Ok(())
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
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
            let _a = state.handle_input(code);
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