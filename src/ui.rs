use std::collections::HashMap;
use std::time::Duration; 
use std::io::stdout;
use std::thread;
use std::sync::{Arc, Mutex};
use std::cmp::Ordering;
use std::env;

// use config::Config;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, read, KeyEvent, poll},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use tui::widgets::{BarChart, ListItem, Paragraph, Table, Wrap};
use tui::layout::Rect;
use rusqlite::Connection;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::Terminal;
use tui::widgets::{List, Block, Borders};
use anyhow::Result;
use crate::db::{self, CardFilter};
use crate::util::*;
use crate::util::views::*;

struct AppState {
    mode: Screen,
    mode_p: Screen,
    title: String,
    deckview: Option<DeckView>,
    settingsview: Option<SettingsView>,
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
    mdc: MakeDeckContents,
    dsi: DeckStatInfo,
    dirty_deck: bool,
    dirty_dbf: bool,
    dirty_cards: Vec<Card>,
    dbc: Arc<Mutex<Connection>>,
    cf: db::CardFilter,
    settings: Settings,
    quit: bool
}

// struct WidgetOwner<'a> {
//     odt: Option<Table<'a>>
// }

impl AppState {
    fn new() -> AppState {
        let p = get_local_file("settings.toml", true);
        let file_settings = FileSettings::new(&p).unwrap();
        let settings = Settings::from(file_settings);
        let p = get_local_file("lieutenant.db", true);
        let conn = Connection::open(p).unwrap();

        db::add_regexp_function(&conn).unwrap();
        let mut app = AppState {
            mode: Screen::MainMenu,
            mode_p: Screen::MainMenu,
            title: String::from("Main Menu"),
            deckview: None,
            settingsview: None,
            deck_id: -1,
            deck: None,
            contents: None,
            sldc: StatefulList::new(),
            slmm: StatefulList::new(),
            sldbc: StatefulList::new(),
            slt: StatefulList::with_items(settings.get_tags()),
            stod: OpenDeckTable::default(),
            ac: None,
            mdc: MakeDeckContents::default(),
            dsi: DeckStatInfo::default(),
            quit: false,
            omnitext: Omnitext::default(),
            dbftext: Omnitext::default(),
            dirty_deck: true,
            dirty_dbf: true,
            dirty_cards: Vec::new(),
            dbc: Arc::new(Mutex::new(conn)),
            cf: CardFilter::new(),
            settings
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
                        if let Some(deck) = self.stod.get() {
                            self.settings.sr(deck.id);
                            self.dirty_deck = true;
                            self.init_deck_view();
                        };
                    }
                    KeyCode::Delete => {
                        self.mode_p = self.mode;
                        self.mode = Screen::Error("Confirm Deletion\nAre you sure you want to delete the below deck?\n{DECK}\nPress Enter to confirm.");
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
                                //TODO: Test against empty string
                                if let Some(name) = self.mdc.commander_names.get(0) {
                                    let c = db::rcfn(&self.dbc.lock().unwrap(), &name, None).unwrap();
                                    match c.is_commander() {
                                        CommanderType::Default => {
                                            let did = db::ideck(
                                                &self.dbc.lock().unwrap(), 
                                                &self.mdc.title, 
                                                &name, 
                                                None,
                                                "Commander").unwrap();
                                            self.settings.sr(did);
                                            self.settings.id(did);
                                            self.mdc = MakeDeckContents::default();
                                            self.init_deck_view(); 
                                        }
                                        CommanderType::Partner => {
                                            self.mdc.commander = name.clone();
                                            self.mdc.commander_names.clear();
                                            self.mdc.focus = MakeDeckFocus::SecondaryCommander;
                                        }
                                        CommanderType::PartnerWith(ss) => {
                                            self.mdc.commander = name.clone();
                                            self.mdc.commander_names = db::rvcnfnp(&self.dbc.lock().unwrap(), &ss).unwrap();
                                            self.mdc.commander2 = ss;
                                            self.mdc.focus = MakeDeckFocus::SecondaryCommander;
                                        }
                                        CommanderType::Invalid => {
                                            self.mode_p = self.mode;
                                            self.mode = Screen::Error("Commander name not found in database.\nPlease check spelling and try again.\nPress Enter to continue.");
                                        }
                                    }
                                }
                            }
                            MakeDeckFocus::SecondaryCommander => {
                                if let Some(name) = self.mdc.commander_names.get(0) {
                                    // match db::rcfn(&self.dbc, &name) {
                                    let c = db::rcfn(&self.dbc.lock().unwrap(), &name, None);
                                    match c {
                                        Ok(_) => {
                                            let did = db::ideck(
                                                &self.dbc.lock().unwrap(), 
                                                &self.mdc.title, 
                                                &self.mdc.commander, 
                                                Some(name.clone()),
                                                "Commander").unwrap();
                                            self.settings.sr(did);
                                            self.settings.id(did);
                                            self.mdc = MakeDeckContents::default();
                                            self.init_deck_view();
                                        }
                                        Err(_) => {
                                            let did = db::ideck(
                                                // &self.dbc, 
                                                &self.dbc.lock().unwrap(), 
                                                &self.mdc.title, 
                                                &self.mdc.commander, 
                                                None,
                                                "Commander").unwrap();
                                            self.settings.sr(did);
                                            self.settings.id(did);
                                            self.mdc = MakeDeckContents::default();
                                            self.init_deck_view();
                                        }
                                    }
                                } else {
                                    let did = db::ideck(
                                        // &self.dbc, 
                                        &self.dbc.lock().unwrap(), 
                                        &self.mdc.title, 
                                        &self.mdc.commander, 
                                        None,
                                        "Commander").unwrap();
                                    self.settings.sr(did);
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
                                self.mdc.commander_names = db::rvcnfn(&self.dbc.lock().unwrap(), &self.mdc.commander).unwrap();
                            }
                            MakeDeckFocus::SecondaryCommander => { 
                                self.mdc.commander2.pop(); 
                                self.mdc.commander_names = db::rvcnfnp(&self.dbc.lock().unwrap(), &self.mdc.commander2).unwrap();
                            }
                        }
                    }
                    KeyCode::Char(ch) => {
                        match self.mdc.focus {
                            MakeDeckFocus::Title => { self.mdc.title.push(ch); }
                            MakeDeckFocus::Commander => { 
                                self.mdc.commander.push(ch); 
                                self.mdc.commander_names = db::rvcnfn(&self.dbc.lock().unwrap(), &self.mdc.commander).unwrap();
                            }
                            MakeDeckFocus::SecondaryCommander => { 
                                self.mdc.commander2.push(ch); 
                                self.mdc.commander_names = db::rvcnfnp(&self.dbc.lock().unwrap(), &self.mdc.commander2).unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Screen::DeckStat => {
                self.mode = Screen::DeckView;
            },
            Screen::Error(s) => {
                match c {
                    KeyCode::Enter => {
                        if s.starts_with("Confirm Deletion") {
                            if let Some(deck) = self.stod.remove() {
                                db::dd(&self.dbc.lock().unwrap(), deck.id).unwrap();
                                self.settings.dd(deck.id);
                            };
                        }
                    }
                    _ => {}
                }
                self.mode = self.mode_p;
            }
            Screen::Settings => {
                if let Some(sv) = &mut self.settingsview {
                    match sv.handle_input(c) {
                        views::SettingsExit::Save(changes) => {
                            if self.mode_p == Screen::MainMenu {
                                self.settings.change(changes, None);
                                self.mode = Screen::MainMenu
                            } else {
                                //TODO: Should this call into the deck view? One source of truth for Deck ID?
                                self.settings.change(changes, Some(self.deck_id));
                                self.mode_p = Screen::Settings;
                                self.mode = Screen::DeckView;
                            }
                        },
                        views::SettingsExit::Hold => {},
                        views::SettingsExit::Cancel => {
                            if self.mode_p == Screen::DeckView {
                                self.mode_p = Screen::Settings;
                                self.mode = Screen::DeckView;
                            } else {
                                self.mode = Screen::MainMenu
                            }

                        },
                    }
                }
            },
            Screen::DeckView => {
                let a = self.deckview.as_mut().unwrap().handle_input(c, &self.dbc.lock().unwrap());
                match a {
                    DeckViewExit::Hold => {},
                    DeckViewExit::MainMenu => self.mode = Screen::MainMenu,
                    DeckViewExit::Stats => self.mode = Screen::DeckStat,
                    DeckViewExit::Settings(did) => {
                        self.mode_p = Screen::DeckView;
                        self.init_settings(Some(did));
                    },
                    DeckViewExit::NewTag(s, did) => self.settings.it(Some(did), s.to_string()),
                }
            },
            
        }

        Ok(())
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
            // Some(Screen::DeckOmni) => { self.init_deck_view(); }
            Some(Screen::Settings) => { self.init_settings(None); }
            // Some(Screen::DeckCard) => {  }
            Some(Screen::MainMenu) => { self.reset(); self.mode = Screen::MainMenu;}
            Some(Screen::DeckView) => { self.init_deck_view(); }
            Some(_) => {}
            None => { self.quit = true }
        }
    }

    fn init_create_view(&mut self) {}

    fn init_deck_view(&mut self) {
        self.deck_id = self.settings.rr();

        if self.dirty_deck {
            let ord = self.settings.rso(Some(self.deck_id));
            self.contents = Some(db::rvcfdid(&self.dbc.lock().unwrap(), self.deck_id, ord).unwrap());
            self.dirty_deck = false;
            self.dirty_cards = Vec::new();
            self.sldc = StatefulList::with_items(self.contents.clone().unwrap());
            self.sldc.next();
        }

        // self.deck = Some(db::rdfdid(&self.dbc, self.deck_id).unwrap());
        self.deck = Some(db::rdfdid(&self.dbc.lock().unwrap(), self.deck_id).unwrap());
        self.slt = StatefulList::with_items(self.settings.get_tags_deck(self.deck_id));
        self.slt.next();

        if let Some(c) = self.sldc.get_string() {
            // self.ac = Some(db::rcfndid(&self.dbc, &c, self.deck_id).unwrap());
            self.ac = Some(db::rcfndid(&self.dbc.lock().unwrap(), &c, self.deck_id).unwrap());
        } else {
            self.ac = None;
        }

        let ord = self.settings.rso(Some(self.deck_id));
        let df = self.settings.rdf(Some(self.deck_id));

        self.cf = CardFilter::from(self.deck_id, &self.deck.as_ref().unwrap().color, df, ord);
        
        self.deckview = Some(DeckView::new(
            self.deck_id, 
            &self.dbc.lock().unwrap(), 
            self.settings.get_tags_deck(self.deck_id),
            df,
            ord
        ));
        self.mode = Screen::DeckView;
    }
    
    fn init_settings(&mut self, odid: Option<i32>) {
        let sv = match odid {
            Some(did) => {
                views::SettingsView::new(
                    self.settings.get_tags_deck(did), 
                    self.settings.rdf(Some(did)), 
                    self.settings.rso(Some(did)), 
                    String::from("Deck Settings"), None)
            },
            None => { 
                views::SettingsView::new(
                    self.settings.get_tags(), 
                    self.settings.rdf(None), 
                    self.settings.rso(None), 
                    String::from("Global Settings"), Some(false))
            },
        };
        self.settingsview = Some(sv);
        self.mode = Screen::Settings;
    }
    
    fn init_open_view(&mut self) {
        self.mode = Screen::OpenDeck;
        self.stod.init(&self.dbc.lock().unwrap());
        // let vd = db::rvd(&self.dbc).unwrap();
        // self.slod = StatefulList::with_items(vd);
        // self.slod.next();
    }

    fn init_main_menu(&mut self) {
        let mut items = Vec::new();
        if self.settings.rr() > 0 { items.push(MainMenuItem::from_with_screen(String::from("Load most recent deck"), Screen::DeckView)); }
        items.push(MainMenuItem::from_with_screen(String::from("Create a new deck"), Screen::MakeDeck));
        items.push(MainMenuItem::from_with_screen(String::from("Load a deck"), Screen::OpenDeck));
        items.push(MainMenuItem::from_with_screen(String::from("Settings"), Screen::Settings));
        items.push(MainMenuItem::from(String::from("Quit")));
        
        self.slmm = StatefulList::with_items(items);
        self.slmm.next();
    }

    //TODO: See about refactoring away?
    pub fn rvli(&self) -> Vec<ListItem> {
        self.slmm.rvli()
    }

    //TODO: See about refactoring away?
    pub fn get_main_cards(&self) -> Vec<CardStat> {
        db::rvmcfd(&self.dbc.lock().unwrap(), self.deck_id).unwrap()
    }
    
    //TODO: See about refactoring into DeckView?
    pub fn generate_dss_info(& self) -> DeckStatInfo {
        let mut dsi = DeckStatInfo::default();
        let vc = db::rvmcfd(&self.dbc.lock().unwrap(), self.deck_id).unwrap();
        let types = vec!["Legendary", "Land", "Creature", "Planeswalker", "Enchantment", "Instant", "Sorcery", "Artifact"];
        let mut hm_type: HashMap<String, u64> = HashMap::new();
        let mut hm_cmc: HashMap<String, u64> = HashMap::new();
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
        for (k, v) in hm_type {
            dsi.type_data.push((k.clone(), v));
        }
        for (k, v) in hm_tag {
            dsi.tag_data.push((k.clone(), v));
        }

        dsi.cmc_data.sort_by(|a, b| a.0.cmp(&b.0));
        dsi.price_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        dsi.type_data.sort_by(|a, b| a.0.cmp(&b.0));
        dsi.tag_data.sort_by(|a, b| {
            let o = b.1.cmp(&a.1);
            if o == Ordering::Equal { return a.0.cmp(&b.0) }
            o
        });

        dsi
    }

    pub fn generate_deck_table(&mut self) -> Table {
        self.stod.rdt()
    }

    pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
        match self.mode {
            Screen::DeckView => self.deckview.as_ref().unwrap().render(frame),
            Screen::Settings => self.settingsview.as_ref().unwrap().render(frame),
            _ => todo!(),
        }
    }
}

fn generate_deckstat_managroup<'a>(vcs: &'a Vec<CardStat>) -> List<'a> {
    let mut cc = HashMap::new();
    let mut total = 0;
    for cs in vcs {
        for ch in cs.mana_cost.chars() {
            if ['W', 'U', 'B', 'R', 'G', 'C', 'X'].contains(&ch) {
                if let Some(i) = cc.get_mut(&ch) { *i += 1; }
                else { cc.insert(ch, 1); }
                total += 1;
            }
        }
    }
    let vli: Vec<ListItem> = cc
        .drain()
        .sorted()
        .map(|(symbol, amount)| { 
            let percentage: f64 = 100.0 * (amount as f64) / (total as f64);
            ListItem::new(format!("{}: {} ({:.1}%)", symbol, amount, percentage ))
        }).collect();
    List::new(vli).block(Block::default().title("Mana Colors").borders(Borders::ALL))
}

fn generate_deckstat_recommendations<'a>(vcs: &'a Vec<CardStat>) -> Paragraph<'a> {
    let mut nonlands = 0;
    let mut total_cmc: u16 = 0;
    let mut cc = HashMap::new();
    let mut recs = Vec::new();

    for cs in vcs {
        total_cmc += cs.cmc as u16;
        if let Some(i) = cc.get_mut(&cs.cmc) { *i += 1; }
        else { cc.insert(cs.cmc, 1); }
        // TODO: check for modal cards/double-facing.
        if !cs.types.contains("Land") { nonlands += 1; }
        // TODO: Add check for legalities after adding them to CardStat
    }

    if nonlands < 60 { 
        recs.push(format!("Only {} nonland cards in deck! Consider adding more.", nonlands)); 
    } else if nonlands > 70 { 
        recs.push(format!("{} nonland cards in deck! Is that too many?", nonlands)); 
    } else {
        recs.push(format!("{} nonland cards in deck.", nonlands));
    }
    let avg_cmc: f64 = ((total_cmc as f64) / (nonlands as f64)).into();
    if avg_cmc > 4.0 { 
        recs.push(format!("Average mana cost {:.2}. Seems high.", avg_cmc)); 
    } else if avg_cmc < 3.0 { 
        recs.push(format!("Average mana cost {:.2}. Seems low.", avg_cmc)); 
    } else {
        recs.push(format!("Average mana cost {:.2}.", avg_cmc)); 
    }

    Paragraph::new(recs.join("\n")).block(Block::default().title("Deck Notes").borders(Borders::ALL)).wrap(Wrap {trim: false})
}

fn generate_deckstat_manacurve<'a>(vcs: &'a Vec<CardStat>) -> Vec<(&str, u64)> {
    let mut hm_cmc: HashMap<&str, u64> = HashMap::new();
    hm_cmc.insert("0", 0);
    hm_cmc.insert("1", 0);
    hm_cmc.insert("2", 0);
    hm_cmc.insert("3", 0);
    hm_cmc.insert("4", 0);
    hm_cmc.insert("5", 0);
    hm_cmc.insert("6", 0);
    hm_cmc.insert("7+", 0);

    for cs in vcs {
        match cs.cmc {
            0 => { if !cs.types.contains("Land") { let a = hm_cmc.get_mut("0").unwrap(); *a += 1; } }
            1 => { let a = hm_cmc.get_mut("1").unwrap();  *a += 1; }
            2 => { let a = hm_cmc.get_mut("2").unwrap();  *a += 1; }
            3 => { let a = hm_cmc.get_mut("3").unwrap();  *a += 1; }
            4 => { let a = hm_cmc.get_mut("4").unwrap();  *a += 1; }
            5 => { let a = hm_cmc.get_mut("5").unwrap();  *a += 1; }
            6 => { let a = hm_cmc.get_mut("6").unwrap();  *a += 1; }
            _ => { let a = hm_cmc.get_mut("7+").unwrap();  *a += 1;}
        }
    }

    let data: Vec<(&str, u64)> = hm_cmc.drain().sorted_by_key(|x|  x.0).map(|(k, v)| (k, v)).collect();
    data
}

fn generate_deckstat_barchart<'a>(title: &'a str, vd: &'a Vec<(&'a str, u64)>) -> BarChart<'a> {
    BarChart::default()
        .block(Block::default().title(title).borders(Borders::ALL))
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::White))
        .value_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .label_style(Style::default().fg(Color::Cyan))
        .data(vd.as_slice().clone())
}

fn draw<'a>(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, 
    state: &mut AppState,) -> Result<()> {
    let _a = terminal.draw(|f| {

        let chunks = match state.mode {
            Screen::MainMenu | Screen::OpenDeck => {
                Layout::default().constraints([Constraint::Percentage(100)]).split(f.size())
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
                    .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)].as_ref())
                    .split(chunks[0]);

                let mut bottom_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)].as_ref())
                    .split(chunks[1]);

                top_chunks.append(&mut bottom_chunks);
                top_chunks
            }
            Screen::DeckView => Vec::new(),
        };        
            
        match state.mode {
            Screen::MainMenu => {
                let list = List::new(state.rvli())
                    .block(Block::default().title("Main Menu").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                
                f.render_stateful_widget(list, chunks[0], &mut state.slmm.state.clone());
            }
            Screen::OpenDeck => {
                let mut ts = state.stod.state.clone();
                let table = state.generate_deck_table();
                    
                f.render_stateful_widget(table, chunks[0], &mut ts);
            }
            Screen::Settings => state.render(f),
            Screen::Error(s) => {
                let (title, mut message) = s.split_once("\n").unwrap();
                let s = message.replace("{DECK}", state.stod.get().unwrap().name.as_str());
                if title == "Confirm Deletion" {
                    message = s.as_str();
                }
                let err_message = Paragraph::new(message)
                    .block(Block::default().borders(Borders::ALL).title(title));
                let area = centered_rect(60, f.size());
                // f.render_widget(Clear, area); //Not necessary because we clear the full screen anyway on every input.
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
            Screen::DeckStat => {
                let dsi = state.generate_dss_info();
                let vcs = state.get_main_cards();
                
                let cmc_data: Vec<(&str, u64)> = dsi.cmc_data.iter()
                    .map(|(k, v)| (k.as_str(), *v))
                    .collect();
                let type_data: Vec<(&str, u64)> = dsi.type_data.iter()
                    .map(|(k, v)| (k.as_str(), *v))
                    .collect();
                let tag_data: Vec<ListItem> = dsi.tag_data.iter()
                    .map(|(k, v)| ListItem::new(format!("{}: {}", k, v)))
                    .collect(); 

                let dss = DeckStatScreen::from(&cmc_data, &dsi.price_data, &type_data, tag_data);
                let mg = generate_deckstat_managroup(&vcs);
                let recs = generate_deckstat_recommendations(&vcs);
                let mc = generate_deckstat_manacurve(&vcs);
                let mcc = generate_deckstat_barchart("Mana Values", &mc);

                f.render_widget(mcc, chunks[0]);
                f.render_widget(dss.prices, chunks[1]);
                f.render_widget(mg, chunks[2]);
                f.render_widget(dss.type_breakdown, chunks[3]);
                f.render_widget(dss.tag_list, chunks[4]);
                f.render_widget(recs, chunks[5]);
            }
            Screen::DeckView => state.render(f),
        }
    })?;
    Ok(())
}

fn centered_rect(percent_x: u16, r: Rect) -> Rect {
    
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(r.height/2 - 3),
                Constraint::Length(5),
                Constraint::Length(r.height/2 - 2),
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
                    let arc = Arc::clone(&state.dbc);
                    thread::spawn(move || {
                        db::ucfd(&arc, did).unwrap();
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

    let mut p = env::current_exe().unwrap();
    p.pop();
    p.push("settings.toml");
    std::fs::write(p, state.settings.to_toml()).unwrap();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}