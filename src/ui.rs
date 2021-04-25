use std::collections::HashMap;
use std::convert::TryInto;
use std::time::Duration; 
use std::io::stdout;
use std::thread;
use std::sync::{Arc, RwLock};
// use config::Config;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, read, KeyEvent, poll},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::widgets::{Clear, ListItem, Paragraph, Table};
use tui::layout::Rect;
use rusqlite::Connection;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::Terminal;
use tui::widgets::{List, Block, Borders};
use anyhow::Result;
// use crate::{Card, Deck, db::rcfndid};
use crate::db;
use crate::util::*;
// mod util;
// use crate::util::{StatefulList, MainMenuItem, Screen, 
//     DeckScreen, MakeDeckScreen, MakeDeckContents, 
//     Omnitext, DeckStatInfo, DeckStatScreen, 
//     MakeDeckFocus, Card, Deck,
//     OpenDeckTable, Settings};

struct AppState {
    mode: Screen,
    mode_p: Screen,
    title: String,
    deck_id: i32,
    deck: Option<Deck>,
    contents: Option<Vec<Card>>,
    omnitext: Omnitext,
    dbftext: Omnitext,
    sldc: StatefulList<Card>,
    slmm: StatefulList<MainMenuItem>,
    sldbc: StatefulList<Card>,
    slt: StatefulList<String>,
    stod: OpenDeckTable,
    ac: Option<Card>,
    tag: String,
    mdc: MakeDeckContents,
    dsi: DeckStatInfo,
    dirty_deck: bool,
    dirty_dbf: bool,
    dirty_cards: Vec<Card>,
    dbc: Connection,
    dbc_mutex: Arc<RwLock<Connection>>,
    config: Settings,
    quit: bool
}

struct WidgetOwner<'a> {
    odt: Option<Table<'a>>
}

impl AppState {
    fn new() -> AppState {
        let conn = Connection::open("lieutenant.db").unwrap();
        let conn2 = Connection::open("lieutenant.db").unwrap();
        db::add_regexp_function(&conn).unwrap();
        let config = Settings::new().unwrap();
        let mut app = AppState {
            mode: Screen::MainMenu,
            mode_p: Screen::MainMenu,
            title: String::from("Main Menu"),
            deck_id: -1,
            deck: None,
            contents: None,
            sldc: StatefulList::new(),
            slmm: StatefulList::new(),
            sldbc: StatefulList::new(),
            slt: StatefulList::with_items(config.get_tags()),
            stod: OpenDeckTable::default(),
            ac: None,
            tag: String::default(),
            mdc: MakeDeckContents::default(),
            dsi: DeckStatInfo::default(),
            quit: false,
            omnitext: Omnitext::default(),
            dbftext: Omnitext::default(),
            dirty_deck: true,
            dirty_dbf: true,
            dirty_cards: Vec::new(),
            dbc: conn,
            dbc_mutex: Arc::new(RwLock::new(conn2)),
            config
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
                    KeyCode::Up => { self.stod.previous(); }
                    KeyCode::Down => { self.stod.next(); }
                    KeyCode::Enter => { 
                        // TODO: Assign correct deck ID to config
                        self.deck_id = self.stod.get().unwrap().id;
                        self.dirty_deck = true;
                        self.init_deck_view();
                    }
                    KeyCode::Delete => {
                        // db::dd(&self.dbc, self.stod.remove().unwrap().id).unwrap();
                        db::dd(&self.dbc_mutex.read().unwrap(), self.stod.remove().unwrap().id).unwrap();
                    }
                    _ => {}
                }
            }
            Screen::DeckOmni => {
                match c {
                    KeyCode::Esc => { self.switch_mode(Some(Screen::MainMenu)); }
                    // KeyCode::Up => { self.slod.previous(); }
                    // KeyCode::Down => { self.slod.next(); }
                    KeyCode::Left => { self.omnitext.left(); }
                    KeyCode::Right => { self.omnitext.right(); }
                    KeyCode::Enter => { 
                        if self.omnitext.text == String::from("/stat") {
                            self.mode = Screen::DeckStat;
                        } else {
                            if self.sldc.items.len() > 0 {
                                self.mode = Screen::DeckCard;
                                if let Some(tag) = self.omnitext.rt() {
                                    self.tag = tag;
                                }
                            }
                        }
                    }
                    KeyCode::Tab => {
                        self.mode = Screen::DbFilter;
                        if self.dirty_deck {
                            // self.contents = Some(db::rvcfdid(&self.dbc, self.deck_id).unwrap());
                            self.contents = Some(db::rvcfdid(&self.dbc_mutex.read().unwrap(), self.deck_id).unwrap());
                            self.dirty_deck = false;
                            self.dirty_cards = Vec::new();
                        }
                        if let Some(c) = self.sldbc.get_string() {
                            // self.ac = Some(db::rcfn(&self.dbc, &c).unwrap());
                            self.ac = Some(db::rcfn(&self.dbc_mutex.read().unwrap(), &c).unwrap());
                        } else {
                            self.ac = None;
                        }
                    }
                    KeyCode::Backspace => { 
                        let cur = self.omnitext.rt();
                        self.omnitext.backspace(); 
                        if cur == None || cur == self.omnitext.rt() {
                            self.uslvc();
                        } 
                    }
                    KeyCode::Delete => { self.omnitext.delete(); self.uslvc(); }
                    KeyCode::Char(c) => {
                        let cur = self.omnitext.rt();
                        self.omnitext.insert(c); 
                        if cur == None || cur == self.omnitext.rt() {
                            self.uslvc(); 
                            if let Some(s) = self.sldc.get_string() {
                                self.ac = Some(db::rcfndid(
                                    // &self.dbc, 
                                    &self.dbc_mutex.read().unwrap(), 
                                    &s, 
                                    self.deck_id).unwrap());
                            } else {
                                self.ac = None;
                            } 
                        }
                    }
                    _ => {}
                }
            }
            Screen::DeckCard => {
                match c {
                    KeyCode::Esc => { self.switch_mode(Some(Screen::MainMenu)); }
                    KeyCode::Up => { 
                        self.ac = Some(db::rcfndid(
                            // &self.dbc, 
                            &self.dbc_mutex.read().unwrap(), 
                            &self.sldc.previous(), 
                            self.deck_id).unwrap()); 
                    }
                    KeyCode::Down => { 
                        self.ac = Some(db::rcfndid(
                            // &self.dbc, 
                            &self.dbc_mutex.read().unwrap(), 
                            &self.sldc.next().unwrap(), 
                            self.deck_id).unwrap());  
                    }
                    KeyCode::Tab => { self.mode = Screen::DeckOmni; }
                    KeyCode::Char(' ') => { self.ac_switch(false); }
                    KeyCode::Delete => { self.remove_from_deck(); }
                    KeyCode::Enter => {
                        if let Some(card) = db::ttindc(
                            // &self.dbc, 
                            &self.dbc_mutex.read().unwrap(), 
                            self.sldc.get().unwrap().name.clone(), 
                            &self.slt.get().unwrap(), 
                            self.deck_id) {
                                self.sldc.replace(card.clone());
                                self.ac = Some(card);
                        };
                    }
                    KeyCode::Right => { self.slt.next(); }
                    KeyCode::Left => { self.slt.previous(); }
                    _ => {}
                }
            }
            Screen::DbFilter => {
                match c {
                    KeyCode::Esc => { self.switch_mode(Some(Screen::MainMenu)); }
                    // KeyCode::Up => { self.slod.previous(); }
                    // KeyCode::Down => { self.slod.next(); }
                    KeyCode::Left => { self.dbftext.left(); }
                    KeyCode::Right => { self.dbftext.right(); }
                    KeyCode::Enter => { 
                        if self.dirty_dbf {
                            self.uslvc();
                            self.dirty_dbf = false;
                        }
                        if self.sldbc.items.len() > 0 {
                            self.mode = Screen::DbCards;
                            // self.ac = Some(db::rcfn(&self.dbc, &self.sldbc.get_string().unwrap()).unwrap());
                            self.ac = Some(db::rcfn(&self.dbc_mutex.read().unwrap(), &self.sldbc.get_string().unwrap()).unwrap());
                        }
                    }
                    KeyCode::Tab => {
                        // self.mode = Screen::DeckOmni;
                        self.switch_mode(Some(Screen::DeckOmni));
                    }
                    KeyCode::Backspace => { self.dbftext.backspace(); self.dirty_dbf = true; }
                    KeyCode::Delete => { self.dbftext.delete(); self.dirty_dbf = true; }
                    KeyCode::Char(c) => {self.dbftext.insert(c); self.dirty_dbf = true; }
                    _ => {}
                }
            }
            Screen::DbCards => {
                match c {
                    KeyCode::Esc => { self.switch_mode(Some(Screen::MainMenu)); }
                    KeyCode::Up => { 
                        self.ac = Some(db::rcfn(
                            // &self.dbc, 
                            &self.dbc_mutex.read().unwrap(), 
                            &self.sldbc.previous()).unwrap());  
                        //TODO: Move to the below model instead
                        // self.sldbc.previous();
                        // self.ac = self.sldbc.get();
                    }
                    KeyCode::Down => { 
                        self.ac = Some(db::rcfn(
                            // &self.dbc, 
                            &self.dbc_mutex.read().unwrap(), 
                            &self.sldbc.next().unwrap()).unwrap()); 
                        //TODO: Move to the below model instead
                        // self.sldbc.next();
                        // self.ac = self.sldbc.get();
                     }
                    KeyCode::Tab => { self.mode = Screen::DbFilter; }
                    KeyCode::Enter => {
                        let card = self.sldbc.get().unwrap().clone();
                        // if db::cindid(&self.dbc, &card.name, self.deck_id) {
                        if db::cindid(&self.dbc_mutex.read().unwrap(), &card.name, self.deck_id) {
                            if let Some(card) = db::ttindc(
                                // &self.dbc, 
                                &self.dbc_mutex.read().unwrap(), 
                                self.sldbc.get().unwrap().name.clone(), 
                                &self.slt.get().unwrap(), 
                                self.deck_id) {
                                    self.sldbc.replace(card.clone());
                                    self.ac = Some(card);
                            };
                        } else {
                            // let mut dc = db::ictodc(&self.dbc, &card, self.deck_id.try_into().unwrap()).unwrap();
                            let mut dc = db::ictodc(&self.dbc_mutex.read().unwrap(), &card, self.deck_id.try_into().unwrap()).unwrap();
                            self.dirty_deck = true;
                            self.dirty_cards.append(&mut dc);
                        }
                    }
                    KeyCode::Delete => { self.remove_from_deck(); }
                    KeyCode::Char(' ') => { self.ac_switch(true); }
                    KeyCode::Right => { self.slt.next(); }
                    KeyCode::Left => { self.slt.previous(); }
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
                                //TODO: Test against empty string
                                if let Some(name) = self.mdc.commander_names.get(0) {
                                    // match db::rcfn(&self.dbc, &name) {
                                    let c = self.dbc_mutex.read().unwrap();
                                    match db::rcfn(&c, &name) {
                                        Ok(card) => {
                                            if card.text.contains("Partner") {
                                                self.mdc.commander = name.clone();
                                                self.mdc.commander_names.clear();
                                                self.mdc.focus = MakeDeckFocus::SecondaryCommander;
                                            } else {
                                                self.deck_id = db::ideck(
                                                    // &self.dbc, 
                                                    &self.dbc_mutex.read().unwrap(), 
                                                    &self.mdc.title, 
                                                    &name, 
                                                    None,
                                                    "Commander").unwrap();
                                                self.mdc = MakeDeckContents::default();
                                                self.init_deck_view();
                                            }
                                        }
                                        Err(_) => {
                                            self.mode_p = self.mode;
                                            self.mode = Screen::Error("Commander name not found in database.\nPlease check spelling and try again.\nPress Enter to continue.");
                                        }
                                    }
                                }
                                
                            }
                            MakeDeckFocus::SecondaryCommander => {
                                if let Some(name) = self.mdc.commander_names.get(0) {
                                    // match db::rcfn(&self.dbc, &name) {
                                    match db::rcfn(&self.dbc_mutex.read().unwrap(), &name) {
                                        Ok(_) => {
                                            self.deck_id = db::ideck(
                                                // &self.dbc, 
                                                &self.dbc_mutex.read().unwrap(), 
                                                &self.mdc.title, 
                                                &self.mdc.commander, 
                                                Some(name),
                                                "Commander").unwrap();
                                            self.mdc = MakeDeckContents::default();
                                            self.init_deck_view();
                                        }
                                        Err(_) => {
                                            self.deck_id = db::ideck(
                                                // &self.dbc, 
                                                &self.dbc_mutex.read().unwrap(), 
                                                &self.mdc.title, 
                                                &self.mdc.commander, 
                                                None,
                                                "Commander").unwrap();
                                            self.mdc = MakeDeckContents::default();
                                            self.init_deck_view();
                                        }
                                    }
                                } else {
                                    self.deck_id = db::ideck(
                                        // &self.dbc, 
                                        &self.dbc_mutex.read().unwrap(), 
                                        &self.mdc.title, 
                                        &self.mdc.commander, 
                                        None,
                                        "Commander").unwrap();
                                    self.mdc = MakeDeckContents::default();
                                    self.init_deck_view();
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => { 
                        match self.mdc.focus {
                            MakeDeckFocus::Title => { self.mdc.title.pop(); }
                            MakeDeckFocus::Commander => { 
                                self.mdc.commander.pop(); 
                                // self.mdc.commander_names = db::rvcnfn(&self.dbc, &self.mdc.commander).unwrap();
                                self.mdc.commander_names = db::rvcnfn(&self.dbc_mutex.read().unwrap(), &self.mdc.commander).unwrap();
                            }
                            MakeDeckFocus::SecondaryCommander => { 
                                self.mdc.commander2.pop(); 
                                // self.mdc.commander_names = db::rvcnfnp(&self.dbc, &self.mdc.commander2).unwrap();
                                self.mdc.commander_names = db::rvcnfnp(&self.dbc_mutex.read().unwrap(), &self.mdc.commander2).unwrap();
                            }
                        }
                    }
                    KeyCode::Char(ch) => {
                        match self.mdc.focus {
                            MakeDeckFocus::Title => { self.mdc.title.push(ch); }
                            MakeDeckFocus::Commander => { 
                                self.mdc.commander.push(ch); 
                                // self.mdc.commander_names = db::rvcnfn(&self.dbc, &self.mdc.commander).unwrap();
                                self.mdc.commander_names = db::rvcnfn(&self.dbc_mutex.read().unwrap(), &self.mdc.commander).unwrap();
                            }
                            MakeDeckFocus::SecondaryCommander => { 
                                self.mdc.commander2.push(ch); 
                                // self.mdc.commander_names = db::rvcnfnp(&self.dbc, &self.mdc.commander2).unwrap();
                                self.mdc.commander_names = db::rvcnfnp(&self.dbc_mutex.read().unwrap(), &self.mdc.commander2).unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Screen::DeckStat => {
                match c {
                    _ => { self.mode = Screen::DeckOmni; }
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

    fn remove_from_deck(&mut self) {
        // let card = self.sldc.get().unwrap().clone();
        let card = match self.mode {
            Screen::DeckCard => { self.sldc.get().unwrap().clone() }
            Screen::DbCards => { self.sldbc.get().unwrap().clone() }
            _ => { Card::default() }
        };
        match &card.lo {
            crate::util::Layout::Flip(_, n) | 
            crate::util::Layout::Split(_, n) | 
            crate::util::Layout::ModalDfc(_, n) | 
            crate::util::Layout::Aftermath(_, n) | 
            crate::util::Layout::Adventure(_, n) | 
            crate::util::Layout::Transform(_, n) => { 
                self.sldc.remove_named(&n); 
                db::dcntodc(&self.dbc, &n, self.deck_id).unwrap();
            }
            crate::util::Layout::Meld(s, n, m) => { 
                self.sldc.remove_named(&m);
                // This should be the only database call that could result in an error.
                let _a = db::dcntodc(&self.dbc, &m, self.deck_id);
                if s == &'b' { 
                    self.sldc.remove_named(&n); 
                    db::dcntodc(&self.dbc, &n, self.deck_id).unwrap();
                } 
            }
            _ => {}
        }
        db::dcntodc(&self.dbc, &card.name, self.deck_id).unwrap();
        match self.mode {
            Screen::DeckCard => {
                if let Some(s) = self.sldc.remove() {
                    self.ac = Some(db::rcfndid(
                        &self.dbc, 
                        &s, 
                        self.deck_id).unwrap());
                } else {
                    self.ac = None;
                    self.mode = Screen::DeckOmni;
                }
            }
            Screen::DbCards => {
                //TODO: Finish this
                self.dirty_cards = self.dirty_cards.iter().filter(|&c| c.ne(&card)).cloned().collect();
            }
            _ => {}
        }
        
        self.dirty_deck = true;
    }

    fn reset(&mut self) {
        self.deck_id = -1;
        self.ac = None;
        self.omnitext = Omnitext::default();
        self.dbftext = Omnitext::default();
        self.contents = None;
        self.mdc = MakeDeckContents::default();
        self.sldc = StatefulList::new();
        self.sldbc = StatefulList::new();
        self.dirty_deck = true;
        self.dirty_dbf = true;
    }

    fn switch_mode(&mut self, next: Option<Screen>) {
        match next {
            Some(Screen::MakeDeck) => { self.mode = Screen::MakeDeck; }
            Some(Screen::OpenDeck) => { self.init_open_view(); }
            Some(Screen::DeckOmni) => { /* Set deck_id here */ self.init_deck_view(); }
            Some(Screen::Settings) => { self.init_settings(); }
            Some(Screen::DeckCard) => {  }
            Some(Screen::MainMenu) => { self.reset(); self.mode = Screen::MainMenu;}
            Some(_) => {}
            None => { self.quit = true }
        }
    }

    fn init_create_view(&mut self) {}

    fn init_deck_view(&mut self) {
        if self.dirty_deck {
            // self.contents = Some(db::rvcfdid(&self.dbc, self.deck_id).unwrap());
            self.contents = Some(db::rvcfdid(&self.dbc_mutex.read().unwrap(), self.deck_id).unwrap());
            self.dirty_deck = false;
            self.dirty_cards = Vec::new();
            self.sldc = StatefulList::with_items(self.contents.clone().unwrap());
            self.sldc.next();
        }

        // self.deck = Some(db::rdfdid(&self.dbc, self.deck_id).unwrap());
        self.deck = Some(db::rdfdid(&self.dbc_mutex.read().unwrap(), self.deck_id).unwrap());
        self.slt = StatefulList::with_items(self.config.get_tags_deck(self.deck_id as usize));
        self.slt.next();

        if let Some(c) = self.sldc.get_string() {
            // self.ac = Some(db::rcfndid(&self.dbc, &c, self.deck_id).unwrap());
            self.ac = Some(db::rcfndid(&self.dbc_mutex.read().unwrap(), &c, self.deck_id).unwrap());
        } else {
            self.ac = None;
        }
        
        self.mode = Screen::DeckOmni;
    }
    
    fn init_settings(&mut self) {}
    
    fn init_open_view(&mut self) {
        self.mode = Screen::OpenDeck;
        self.stod.init(&self.dbc);
        // let vd = db::rvd(&self.dbc).unwrap();
        // self.slod = StatefulList::with_items(vd);
        // self.slod.next();
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

    fn ac_switch(&mut self, general: bool) {
        let mut rel = String::new();
        let mut rel2 = String::new();
        let mut side = &'a';
        if let Some(card) = &self.ac {
            match &card.lo {
                crate::util::Layout::Flip(_, n) | 
                crate::util::Layout::Split(_, n) | 
                crate::util::Layout::ModalDfc(_, n) | 
                crate::util::Layout::Aftermath(_, n) | 
                crate::util::Layout::Adventure(_, n) | 
                crate::util::Layout::Transform(_, n) => { rel = n.clone() }
                crate::util::Layout::Meld(s, n, m) => { side = s; rel = n.clone(); rel2 = m.clone(); }
                _ => { return; }
            }
        }
        
        if rel2 != String::new() {
            if side == &'a' {
                // let meld = db::rcfn(&self.dbc, &rel2).unwrap();
                let meld = db::rcfn(&self.dbc_mutex.read().unwrap(), &rel2).unwrap();
                match meld.lo {
                    crate::util::Layout::Meld(_, face, _) => {
                        if face.clone() == rel {
                            rel = rel2;
                        }
                    }
                    _ => {}
                }
            }
        }
        if general {
            // self.ac = Some(db::rcfn(&self.dbc, &rel).unwrap());
            self.ac = Some(db::rcfn(&self.dbc_mutex.read().unwrap(), &rel).unwrap());
        } else {
            // if let Ok(c) = db::rcfndid(&self.dbc, &rel, self.deck_id) {
            if let Ok(c) = db::rcfndid(&self.dbc_mutex.read().unwrap(), &rel, self.deck_id) {
                self.ac = Some(c);
            } else {
                // self.ac = Some(db::rcfn(&self.dbc, &rel).unwrap());
                self.ac = Some(db::rcfn(&self.dbc_mutex.read().unwrap(), &rel).unwrap());
            }
        }
    }

    //TODO: Investigate making this return the first item of the new list for self.ac?
    fn uslvc(&mut self) {

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
        let cf = db::CardFilter::from(&self.deck.as_ref().unwrap(), & ss);
        // let vcr = db::rvcfcf(&self.dbc, cf, general);
        let vcr = db::rvcfcf(&self.dbc_mutex.read().unwrap(), cf, general);
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
            // Screen::OpenDeck => { self.slod.rvli() }
            _ => { Vec::new() }
        }
    }

    pub fn rvlis(&self) -> Vec<ListItem> {
        let mut a = self.contents.clone().unwrap();
        a.append(&mut self.dirty_cards.clone());

        self.sldbc.rvlis(a)
    }

    pub fn get_card_info(&self) -> String {
        match &self.ac {
            Some(card) => { 
                card.ri().join("\n") 
            }
            None => { String::from("No card found!") }
        }
    }

    pub fn generate_dss_info(& self) -> DeckStatInfo {
        let mut dsi = DeckStatInfo::default();
        // let vc = db::rvmcfd(&self.dbc, self.deck_id).unwrap();
        let vc = db::rvmcfd(&self.dbc_mutex.read().unwrap(), self.deck_id).unwrap();
        let types = vec!["Legendary", "Land", "Creature", "Planeswalker", "Enchantment", "Instant", "Sorcery", "Artifact"];
        let mut hm_type: HashMap<String, u64> = HashMap::new();
        let mut hm_cmc: HashMap<String, u64> = HashMap::new();
        // let mut hm_price: HashMap<String, f64> = HashMap::new();
        let mut hm_tag: HashMap<String, u64> = HashMap::new();

        for s in types.clone() { hm_type.insert(String::from(s), 0); }
        for num in 0..7 { hm_cmc.insert((num as u8).to_string(), 0); }
        hm_cmc.insert("7+".to_string(), 0);

        for c in vc {
            for t in c.types.clone().split(" ") {
                match t {
                    "Legendary" => { let a: u64 = hm_type.get(&String::from("Legendary")).unwrap() + 1;  hm_type.insert(String::from("Legendary"), a); }
                    "Land" => { let a: u64 = hm_type.get(&String::from("Land")).unwrap() + 1;  hm_type.insert(String::from("Land"), a); }
                    "Creature" => { let a: u64 = hm_type.get(&String::from("Creature")).unwrap() + 1;  hm_type.insert(String::from("Creature"), a); }
                    "Planeswalker" => { let a: u64 = hm_type.get(&String::from("Planeswalker")).unwrap() + 1;  hm_type.insert(String::from("Planeswalker"), a); }
                    "Enchantment" => { let a: u64 = hm_type.get(&String::from("Enchantment")).unwrap() + 1;  hm_type.insert(String::from("Enchantment"), a); }
                    "Instant" => { let a: u64 = hm_type.get(&String::from("Instant")).unwrap() + 1;  hm_type.insert(String::from("Instant"), a); }
                    "Sorcery" => { let a: u64 = hm_type.get(&String::from("Sorcery")).unwrap() + 1;  hm_type.insert(String::from("Sorcery"), a); }
                    "Artifact" => { let a: u64 = hm_type.get(&String::from("Artifact")).unwrap() + 1;  hm_type.insert(String::from("Artifact"), a); }
                    _ => {}
                }
            }

            match c.cmc {
                0 => { if !c.types.contains("Land") { let a: u64 = hm_cmc.get(&String::from("0")).unwrap() + 1;  hm_cmc.insert(String::from("0"), a); } }
                1 => { let a: u64 = hm_cmc.get(&String::from("1")).unwrap() + 1;  hm_cmc.insert(String::from("1"), a); }
                2 => { let a: u64 = hm_cmc.get(&String::from("2")).unwrap() + 1;  hm_cmc.insert(String::from("2"), a); }
                3 => { let a: u64 = hm_cmc.get(&String::from("3")).unwrap() + 1;  hm_cmc.insert(String::from("3"), a); }
                4 => { let a: u64 = hm_cmc.get(&String::from("4")).unwrap() + 1;  hm_cmc.insert(String::from("4"), a); }
                5 => { let a: u64 = hm_cmc.get(&String::from("5")).unwrap() + 1;  hm_cmc.insert(String::from("5"), a); }
                6 => { let a: u64 = hm_cmc.get(&String::from("6")).unwrap() + 1;  hm_cmc.insert(String::from("6"), a); }
                _ => { let a: u64 = hm_cmc.get(&String::from("7+")).unwrap() + 1;  hm_cmc.insert(String::from("7+"), a);}
            }

            for tag in c.tags {
                if let Some(v) = hm_tag.get_mut(&tag) {
                    let a: u64 = v.checked_add(1).unwrap();
                    hm_tag.insert(tag, a);
                } else {
                    hm_tag.insert(tag, 1);
                }
            }

            dsi.price_data.push((c.name.clone(), c.price));
        }

        hm_tag.remove_entry(&String::from("main"));

        for (k, v) in hm_cmc {
            dsi.cmc_data.push((k.clone(), v));
        }
        // for (k, v) in hm_price {
        //     dsi.price_data.push((k.clone(), v));
        // }
        for (k, v) in hm_type {
            dsi.type_data.push((k.clone(), v));
        }
        for (k, v) in hm_tag {
            dsi.tag_data.push((k.clone(), v));
        }

        dsi.cmc_data.sort_by(|a, b| a.0.cmp(&b.0));
        dsi.price_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        dsi.type_data.sort_by(|a, b| a.0.cmp(&b.0));
        dsi.tag_data.sort_by(|a, b| b.1.cmp(&a.1));

        dsi
    }

    pub fn generate_deck_table(&mut self) -> Table {
        self.stod.rdt()
    }
}

impl<'a> WidgetOwner<'a> {

    pub fn new() -> WidgetOwner<'a> {
        WidgetOwner {
            odt: None
        }
    }
}

fn draw<'a>(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, 
    state: &mut AppState,) -> Result<()> {
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

                vrct.append(&mut Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(20), Constraint::Length(18)].as_ref())
                    .split(cut[0]));
                // vrct.push(cut[0]);
                
                vrct.append(&mut Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(26),Constraint::Min(18)].as_ref())
                    .split(cut[1]));
                vrct
            }
            Screen::MakeDeck => { 
                let mut vrct = Vec::new();
                let cut = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Length(3)])
                    .split(f.size());
                vrct.push(cut[0]);

                let cut = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(26), Constraint::Min(18)])
                    .split(cut[1]);
                vrct.push(cut[1]);

                vrct.append(&mut Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(3)].as_ref())
                    .split(cut[0]));
                vrct
            }
            Screen::Error(_) => { 
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(f.size())
            }
            Screen::Settings => { Vec::new() }
            Screen::DeckStat => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(4)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(f.size());

                let mut top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(chunks[0]);

                let mut bottom_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(chunks[1]);

                top_chunks.append(&mut bottom_chunks);
                top_chunks
            }
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
                let text = state.get_card_info();
                let mut ds = DeckScreen::new(
                    state.dbftext.get_styled(), 
                    &state.slt,
                    state.rvlis(), 
                    text, 
                    state.mode);
                
                ds.focus_omni(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_widget(ds.tags, chunks[1]);
                f.render_stateful_widget(ds.lc, chunks[2], &mut state.sldbc.state.clone());
                f.render_widget(ds.fc, chunks[3]);
            }
            Screen::DeckOmni => {
                // if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = state.get_card_info();
                let mut ds = DeckScreen::new(
                    state.omnitext.get_styled(), 
                    &state.slt,
                    state.rvli(), 
                    text, state.mode);
                
                ds.focus_omni(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_widget(ds.tags, chunks[1]);
                f.render_stateful_widget(ds.lc, chunks[2], &mut state.sldc.state.clone());
                f.render_widget(ds.fc, chunks[3]);
            }
            Screen::OpenDeck => {
                // if widgets.odt == None {
                //     widgets.odt = Some(state.generate_deck_table());
                // }

                let mut ts = state.stod.state.clone();
                let table = state.generate_deck_table();
                    
                f.render_stateful_widget(table, chunks[0], &mut ts);
            }
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
                let list = List::new(mds.potential_commanders.rvli())
                    .block(Block::default().title("Open Deck").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                f.render_widget(mds.title_entry, chunks[0]);
                f.render_stateful_widget(list, chunks[1], &mut mds.potential_commanders.state.clone());
                f.render_widget(mds.commander_entry, chunks[2]);
                f.render_widget(mds.commander2_entry, chunks[3]);
            }
            Screen::DbCards => {
                let text = state.get_card_info();
                let mut ds = DeckScreen::new(
                    state.dbftext.get_styled(), 
                    &state.slt,
                    state.rvlis(), 
                    text, 
                    state.mode);
                
                ds.focus_lc(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_widget(ds.tags, chunks[1]);
                f.render_stateful_widget(ds.lc, chunks[2], &mut state.sldbc.state.clone());
                f.render_widget(ds.fc, chunks[3]);
            }
            Screen::DeckCard => {
                // if chunks.len() < 3 { println!("something went wrong"); state.quit = true; return; }
                let text = state.get_card_info();
                let mut ds = DeckScreen::new(
                    state.omnitext.get_styled(), 
                    &state.slt,
                    state.rvli(), 
                    text, state.mode);
                
                ds.focus_lc(state.mode);
                f.render_widget(ds.omni, chunks[0]);
                f.render_widget(ds.tags, chunks[1]);
                f.render_stateful_widget(ds.lc, chunks[2], &mut state.sldc.state.clone());
                f.render_widget(ds.fc, chunks[3]);
            }
            Screen::DeckStat => {
                let dsi = state.generate_dss_info();
                
                let cmc_data: Vec<(&str, u64)> = dsi.cmc_data.iter()
                    .map(|(k, v)| (k.as_str(), *v))
                    .collect();
                let type_data: Vec<(&str, u64)> = dsi.type_data.iter()
                    .map(|(k, v)| (k.as_str(), *v))
                    .collect();
                let tag_data: Vec<ListItem> = dsi.tag_data.iter()
                    .map(|(k, v)| ListItem::new(format!("{}: {}", k, v)))
                    .collect(); 
                // let price_data = dsi.price_data.iter()
                //     .map(|(n, v)| Row::new(vec![n.as_str(), v.to_string().as_str()]))
                //     .collect();

                let dss = DeckStatScreen::from(&cmc_data, &dsi.price_data, &type_data, tag_data);

                f.render_widget(dss.mana_curve, chunks[0]);
                f.render_widget(dss.prices, chunks[1]);
                f.render_widget(dss.type_breakdown, chunks[2]);
                f.render_widget(dss.tag_list, chunks[3]);
            }
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
    // let mut widgets = WidgetOwner::new();

    loop {
        draw(&mut terminal, &mut state)?;

        if state.mode != Screen::DeckStat {
            if let Event::Key(KeyEvent { code, .. }) = read()? {
                let _a = state.handle_input(code);
                if state.mode == Screen::DeckStat {
                    let did = state.deck_id.clone();
                    thread::spawn(move || {
                        let conn = Connection::open("lieutenant.db").unwrap();
                        db::add_regexp_function(&conn).unwrap();
                        db::ucfd(&conn, did).unwrap();
                    });
                }
            }
        } else {
            if poll(Duration::from_millis(100))? {
                if let Event::Key(KeyEvent { code, .. }) = read()? {
                    let _a = state.handle_input(code);
                }
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