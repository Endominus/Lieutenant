use crossterm::event::KeyCode;
use regex::Regex;
use rusqlite::Connection;
use tui::{layout::{Constraint, Direction, Layout}, text::{Span, Spans}, widgets::{BarChart, Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState}, backend::CrosstermBackend};
use tui::style::{Color, Modifier, Style};

use std::{collections::HashMap, path::PathBuf, env};
use std::cell::RefCell;
use std::rc::Rc;
use serde::Deserialize;
use serde_derive::Serialize;
use config::{Config, ConfigError};
use itertools::Itertools;
use crate::db::{CardFilter, rvcnfcf, ttindc, rcfn, dcntodc};

use self::views::Changes;

pub fn get_local_file(name: &str, file_must_exist: bool) -> PathBuf {
    let mut p = env::current_exe().unwrap();
    p.pop();
    p.push(name);
    if file_must_exist && !p.exists() {
        panic!("Cannot find the {} file. Are you sure it's in the same directory as the executable?", name);
    }
    
    p
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum SortOrder {
    #[default] NameAsc,
    NameDesc,
    CmcAsc,
    CmcDesc,
}

#[derive(PartialEq)]
pub enum CommanderType {
    Default,
    Partner,
    PartnerWith(String),
    Invalid
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum DefaultFilter {
    #[default] Name,
    Text
}

#[derive(Copy, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    MakeDeck,
    OpenDeck,
    Settings,
    DeckView,
    DeckStat,
    Error(&'static str),
}

#[derive(Copy, Clone, PartialEq)]
pub enum DeckViewSection {
    DeckOmni,
    DeckCards,
    DbOmni,
    DbCards
}

pub enum DeckViewExit {
    Hold,
    MainMenu,
    Stats,
    Settings(i32),
    NewTag(String, i32),
}

#[derive(Clone)]
pub enum MakeDeckFocus {
    Title,
    Commander,
    SecondaryCommander,
    // Type
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum CardLayout {
    //TODO: can probably collapse Adventure, Aftermath, etc into one class
    Adventure(char, String),
    Aftermath(char, String),
    Flip(char, String),
    Leveler,
    Meld(char, String, String),
    ModalDfc(char, String),
    #[default] Normal,
    Saga,
    Split(char, String),
    Transform(char, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Relation {
    Single(String),
    Meld {face: String, transform: String },
}

impl ToString for SortOrder {
    fn to_string(&self) -> String {
        match self {
            SortOrder::NameAsc => String::from("+name"),
            SortOrder::NameDesc => String::from("-name"),
            SortOrder::CmcAsc => String::from("+cmc"),
            SortOrder::CmcDesc => String::from("-cmc"),
        }
    }
}

impl ToString for DefaultFilter {
    fn to_string(&self) -> String {
        match self {
            DefaultFilter::Name => String::from("name"),
            DefaultFilter::Text => String::from("text"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileSettings {
    global: FileGlobalSettings,
    decks: HashMap<i32, FileDeckSettings>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileGlobalSettings {
    tags: Vec<String>,
    ordering: String,
    #[serde(rename = "default_filter")]
    df: String,
    version: f64,
    recent: i32,
    open_into_recent: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileDeckSettings {
    tags: Vec<String>,
    ordering: String,
    #[serde(rename = "default_filter")]
    df: String,
}

pub struct DeckView {
    omni: String,
    omniprev: String,
    omnipos: usize,
    vsomni: Vec<String>,
    vcde: Vec<String>,
    vcdec: Vec<String>,
    vcdb: Vec<String>,
    vt: Vec<String>,
    st: usize,
    ac: Option<Card>,
    vcdels: ListState,
    vcdbls: ListState,
    cf: CardFilter,
    dvs: DeckViewSection,
}

#[derive(Debug)]
pub struct Settings {
    global: GlobalSettings,
    decks: Rc<RefCell<HashMap<i32, DeckSettings>>>
}

#[derive(Debug)]
pub struct GlobalSettings {
    tags: Vec<String>,
    ordering: SortOrder,
    df: DefaultFilter,
    version: f64,
    recent: i32,
    open_into_recent: bool,
}

#[derive(Debug)]
pub struct DeckSettings {
    tags: Vec<String>,
    ordering: SortOrder,
    df: DefaultFilter,
}

impl FileSettings {
    pub fn new(path: &PathBuf) -> Result<Self, ConfigError> {
        let mut s = Config::default();
        let tags = Vec::from([
            String::from("main"), 
            String::from("board_wipe"), 
            String::from("draw"), 
            String::from("ramp"), 
            String::from("removal"), 
            String::from("side"), 
        ]);
        let ds: HashMap<String, String> = HashMap::new();
        s.set_default("global.version", 0).unwrap();
        s.set_default("global.recent", -1).unwrap();
        s.set_default("global.open_into_recent", false).unwrap();
        s.set_default("global.tags", tags).unwrap();
        s.set_default("global.ordering", String::from("+name")).unwrap();
        s.set_default("global.default_filter", String::from("name")).unwrap();
        s.set_default("decks", ds).unwrap();
        s.merge(config::File::with_name(path.to_str().unwrap())).unwrap();

        s.try_into()
    }

    // Experimented with using toml_edit, which preserves comments, but found that it didn't preserve indentation.
    pub fn to_toml(&self) -> String {
        let mut vr = Vec::from([String::from("[global]")]);
        vr.push(format!("version = {}", self.global.version));
        vr.push(String::from("tags = ["));
        for t in &self.global.tags {
            vr.push(format!("\t\"{}\",", t));
        }
        vr.push(String::from("]"));
        vr.push(format!("ordering = \"{}\"", self.global.ordering));
        vr.push(format!("default_filter = \"{}\"", self.global.df));
        vr.push(format!("recent = {}", self.global.recent));
        vr.push(format!("open_into_recent = {}", self.global.open_into_recent));
        vr.push(String::from("\n[decks]"));

        // TODO: Explore using a BTreeMap instead to lose dependence on itertools
        let vk = self.decks.keys().sorted();
        
        for k in vk {
            let v = self.decks.get(k).unwrap();
            vr.push(format!("\t[decks.{}]", k));
            vr.push(String::from("\ttags = ["));
            for t in &v.tags {
                vr.push(format!("\t\t\"{}\",", t));
            }
            vr.push(String::from("\t]"));
            vr.push(format!("\tordering = \"{}\"", &v.ordering));
            vr.push(format!("\tdefault_filter = \"{}\"", &v.df));
            vr.push(String::new());
        }
    
        vr.join("\n")
    }
}

impl Settings {
    pub fn get_tags(&self) -> Vec<String> {
        self.global.tags.clone()
    }

    pub fn get_tags_deck(&self, did: i32) -> Vec<String> {
        match self.decks.borrow().get(&did) {
            Some(d) => d.tags.clone(),
            None => self.global.tags.clone(),
        }
    }

    pub fn rso(&self, odid: Option<i32>) -> SortOrder {
        match odid {
            Some(did) => {
                match self.decks.borrow().get(&did) {
                    Some(d) => d.ordering,
                    None => self.global.ordering,
                }
            },
            None => self.global.ordering,
        }
    }

    pub fn rdf(&self, odid: Option<i32>) -> DefaultFilter {
        match odid {
            Some(did) => {
                match self.decks.borrow().get(&did) {
                    Some(d) => d.df,
                    None => self.global.df,
                }
            },
            None => self.global.df,
        }
    }

    pub fn change(&mut self, changes: Changes, odid: Option<i32>) {
        match odid {
            Some(did) => {
                let mut decks = self.decks.borrow_mut();
                if let Some(deck) = decks.get_mut(&did) {
                    deck.df  = changes.df;
                    deck.ordering  = changes.so;
                    for tch in changes.vtch {
                        match tch {
                            views::TagChange::DeleteTag(old) => {
                                if let Some(i) = deck.tags.iter().position(|s| s == &old) {
                                    deck.tags.remove(i);
                                }
                            },
                            views::TagChange::ChangeTag(old, new) => {
                                if let Some(i) = deck.tags.iter().position(|s| s == &old) {
                                    deck.tags.remove(i);
                                    deck.tags.push(new);
                                }
                            },
                            views::TagChange::InsertTag(new) => deck.tags.push(new),
                        }
                        deck.tags.sort();
                    }
                }
            },
            None => {
                self.global.df = changes.df;
                self.global.ordering = changes.so;
                self.global.open_into_recent = changes.oir.unwrap();
                for tch in changes.vtch {
                    match tch {
                        views::TagChange::DeleteTag(old) => {
                            if let Some(i) = self.global.tags.iter().position(|s| s == &old) {
                                self.global.tags.remove(i);
                            }
                        },
                        views::TagChange::ChangeTag(old, new) => {
                            if let Some(i) = self.global.tags.iter().position(|s| s == &old) {
                                self.global.tags.remove(i);
                                self.global.tags.push(new);
                            }
                        },
                        views::TagChange::InsertTag(new) => self.global.tags.push(new),
                    }
                    self.global.tags.sort();
                }
            },
        }
    }

    pub fn rr(&self) -> i32 {
        self.global.recent
    }

    pub fn sr(&mut self, did: i32) {
        self.global.recent = did;
    }

    pub fn it(&mut self, odid: Option<i32>, tag: String) {
        match odid {
            Some(did) => {
                let mut decks = self.decks.borrow_mut();
                match decks.get_mut(&did) {
                    Some(d) => d.add_tag(tag),
                    None => {},
                }
            },
            None => {
                self.global.tags.push(tag);
                self.global.tags.sort();
            },
        };
    }

    pub fn id(&mut self, did: i32) {
        let ds = DeckSettings::duplicate(&self.global);
        let mut decks = self.decks.borrow_mut();
        decks.insert(did, ds);
    }

    pub fn dd(&mut self, deck: i32) {
        self.decks.borrow_mut().remove(&deck);
    }

    pub fn from(fs: FileSettings) -> Self {
        let gs = GlobalSettings::from(fs.global);
        let mut dhash = HashMap::new();

        for (did, fds) in fs.decks {
            let ds = DeckSettings::from(fds);
            dhash.insert(did, ds);
        }

        Self {
            global: gs,
            decks: Rc::from(RefCell::from(dhash)),
        }
    }

    // Experimented with using toml_edit, which preserves comments, but found that it didn't preserve indentation.
    pub fn to_toml(&self) -> String {
        let mut vr = Vec::from([String::from("[global]")]);
        vr.push(format!("version = {}", self.global.version));
        vr.push(String::from("tags = ["));
        for t in &self.global.tags {
            vr.push(format!("\t\"{}\",", t));
        }
        vr.push(String::from("]"));
        vr.push(format!("ordering = \"{}\"", self.global.ordering.to_string()));
        vr.push(format!("default_filter = \"{}\"", self.global.df.to_string()));
        vr.push(format!("recent = {}", self.global.recent));
        vr.push(format!("open_into_recent = {}", self.global.open_into_recent));
        vr.push(String::from("\n[decks]"));

        // TODO: Explore using a BTreeMap instead to lose dependence on itertools
        let decks = self.decks.borrow();
        let vk = decks.keys().sorted();
        
        for k in vk {
            let v = decks.get(k).unwrap();
            vr.push(format!("\t[decks.{}]", k));
            vr.push(String::from("\ttags = ["));
            for t in &v.tags {
                vr.push(format!("\t\t\"{}\",", t));
            }
            vr.push(String::from("\t]"));
            vr.push(format!("\tordering = \"{}\"", &v.ordering.to_string()));
            vr.push(format!("\tdefault_filter = \"{}\"", &v.df.to_string()));
            vr.push(String::new());
        }
    
        vr.join("\n")
    }
}

impl GlobalSettings {
    pub fn from(fgs: FileGlobalSettings) -> Self {
        let df = match fgs.df.as_str() {
            "name" => DefaultFilter::Name,
            "text" => DefaultFilter::Text,
            _ => DefaultFilter::Name,
        };
        
        let ordering = match fgs.ordering.as_str() {
            "+name" => SortOrder::NameAsc,
            "-name" => SortOrder::NameDesc,
            "+cmc" => SortOrder::CmcAsc,
            "-cmc" => SortOrder::CmcDesc,
            _ => SortOrder::NameAsc,
        };

        Self {
            tags: fgs.tags,
            ordering,
            df,
            version: fgs.version,
            recent: fgs.recent,
            open_into_recent: fgs.open_into_recent,
        }
    }

    pub fn toggle_df(&mut self) {
        if self.df == DefaultFilter::Name {
            self.df = DefaultFilter::Text;
        } else {
            self.df = DefaultFilter::Name;
        }
    }
}

impl DeckSettings {
    pub fn from(fds: FileDeckSettings) -> Self {
        let df = match fds.df.as_str() {
            "name" => DefaultFilter::Name,
            "text" => DefaultFilter::Text,
            _ => DefaultFilter::Name,
        };
        
        let ordering = match fds.ordering.as_str() {
            "+name" => SortOrder::NameAsc,
            "-name" => SortOrder::NameDesc,
            "+cmc" => SortOrder::CmcAsc,
            "-cmc" => SortOrder::CmcDesc,
            _ => SortOrder::NameAsc,
        };
        
        Self {
            tags: fds.tags,
            ordering,
            df,
        }
    }

    pub fn duplicate(gs: &GlobalSettings) -> Self {
        Self {
            tags: gs.tags.clone(),
            ordering: gs.ordering,
            df: gs.df,
        }
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.tags.sort();
        }
    }

    pub fn toggle_df(&mut self) {
        if self.df == DefaultFilter::Name {
            self.df = DefaultFilter::Text;
        } else {
            self.df = DefaultFilter::Name;
        }
    }
}

pub struct StatefulList<T: ToString + PartialEq> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T: ToString + PartialEq> StatefulList<T> {

    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) -> Option<String> {
        if self.items.len() > 0 {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
            Some(self.items.get(i).unwrap().to_string())
        } else {
            None
        }
    }

    pub fn previous(&mut self) -> String {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.items.get(i).unwrap().to_string()
    }
    
    pub fn select(&mut self, selected: &T) {
        let mut i = 0;
        
        for item in &self.items {
            if item == selected {
                self.state.select(Some(i));
                break;
            }
            i += 1;
        }
    }
    
    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn get(& self) -> Option<&T> {
        // There should be a more elegant way of doing this.
        if self.items.len() > 0 {
            let a = self.items.get(self.state.selected().unwrap()).unwrap();
            return Some(a);
        }
        None
    }

    pub fn get_string(& self) -> Option<String> {
        if self.items.len() > 0 {
            let a = self.items.get(self.state.selected().unwrap()).unwrap();
            return Some(a.to_string());
        }
        None
    }

    pub fn remove(&mut self) -> Option<String> {
        let mut a = self.state.selected().unwrap();
        self.items.remove(a);
        if self.items.len() > 0 {
            if a == self.items.len() { a -= 1; }
            self.state.select(Some(a));
            Some(self.items.get(a).unwrap().to_string().clone())
        } else {
            None
        }
    }

    pub fn remove_named(&mut self, s: &String) {
        let mut i = 0;
        let a = self.state.selected().unwrap();
        for item in &self.items {
            if &item.to_string() == s {
                break;
            }
            i += 1;
        }
        if i < self.items.len() {
            self.items.remove(i);
            if i < a {
                self.state.select(Some(a-1));
            }
        }
    }

    pub fn replace(&mut self, obj: T) {
        let a = self.state.selected().unwrap();
        self.items.remove(a);
        self.items.insert(a, obj);
    }

    pub fn rvli(& self) -> Vec<ListItem> {
        self.items.iter().map(|f| ListItem::new(f.to_string())).collect()
    }

    pub fn rvlis(& self,  pl: Vec<Card>) -> Vec<ListItem> {
        let vs: Vec<String> = pl.iter().map(|f| f.to_string()).collect();

        self.items.iter().map(|f| 
            if vs.contains(&f.to_string()) {
                ListItem::new(f.to_string()).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))
            } else {
                ListItem::new(f.to_string())
            }
        ).collect()
    }

}

#[derive(Default)]
pub struct OpenDeckTable {
    decks: Vec<Deck>,
    pub state: TableState
}

impl OpenDeckTable {
    pub fn init(&mut self, conn: &Connection) {
        let decks = crate::db::rvd(conn).unwrap();
        if decks.len() > 0 {
            self.state.select(Some(0));
        }
        self.decks = decks;
    }
    
    //TODO: Could cause issues when using a new db; test!
    pub fn rdt(&self) -> Table {
        let decks = self.decks.clone();
        let headers = Row::new(vec![
            Cell::from("ID"),
            Cell::from("Deck Name"),
            Cell::from("Commander(s)"),
            Cell::from("Color"),
        ])
        .style(Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Cyan));
        let mut rows = Vec::new();

        for deck in decks {
            let (height, com2) = match deck.commander2 {
                Some(c) => { (2, c.name) }
                None => { (1, String::new()) }
            };

            let r = Row::new(vec![
                Cell::from(deck.id.to_string()),
                Cell::from(deck.name),
                Cell::from(format!("{}\n{}", deck.commander.name, com2)),
                Cell::from(deck.color)
                ])
                .height(height)
                .style(Style::default());

            rows.push(r);
        }

        let table = Table::new(rows)
            .header(headers)
            .block(Block::default()
                .borders(Borders::ALL))
            .widths(&[Constraint::Length(4), Constraint::Percentage(40), Constraint::Percentage(40), Constraint::Length(7)])
            .column_spacing(1)
            .highlight_style(Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC));

        table
    }

    pub fn next(&mut self) {
        if self.decks.len() == 0 { return }
        if self.decks.len() > 0 {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.decks.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    pub fn previous(&mut self) {
        if self.decks.len() == 0 { return }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.decks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn get(&self) -> Option<&Deck> {
        if self.decks.len() == 0 { return None }
        if let Some(i) = self.state.selected() {
            self.decks.get(i)
        } else {
            None
        }
    }

    pub fn remove(&mut self) -> Option<Deck> {
        if self.decks.len() == 0 { return None }
        let a = self.state.selected().unwrap();
        let d = self.decks.remove(a);

        if self.decks.len() == 0 {
            self.state = TableState::default();
        } else if self.decks.len() == a { self.state.select(Some(a-1)); }

        Some(d)
    }
}

#[derive(Default)]
pub struct Omnitext {
    pub text: String,
    position: usize,
}

impl Omnitext {
    pub fn left(&mut self) {
        if self.position > 0 {
            self.position -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.position < self.text.len() {
            self.position += 1;
        }
    }
    
    pub fn insert(&mut self, c: char) {
        self.text.insert(self.position, c);
        self.position += 1;
    }

    pub fn delete(&mut self) {
        if self.position < self.text.len() {
            self.text.remove(self.position);
        }
    }

    pub fn backspace(&mut self) {
        if self.position > 0 {
            self.text.remove(self.position - 1);
            self.position -= 1;
        }
    }

    pub fn get(&self) -> String {
        self.text.clone().to_lowercase()
    }

    pub fn get_styled(&self) -> Spans {
        let spans = if self.position < self.text.len() {
            let (s1, s2) = self.text.split_at(self.position);
            let (s2, s3) = s2.split_at(1);
            vec![
                Span::styled(s1, Style::default()),
                Span::styled(s2, Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::styled(s3, Style::default()),
            ]
        } else {
            vec![
                Span::styled(self.text.as_str(), Style::default()),
                Span::styled(" ", Style::default().add_modifier(Modifier::UNDERLINED)),
            ]
        };

        Spans::from(spans)
    }

    pub fn rt(&self) -> Option<String> {
        let re = Regex::new(r"/tag:(\w*)").unwrap();
        if let Some(cap) = re.captures(self.text.as_str()) { return Some(String::from(&cap[1])) }
        None
    }
}

#[derive(PartialEq)]
pub struct MainMenuItem {
    pub text: String,
    pub next: Option<Screen>,
}

impl MainMenuItem {
    pub fn from(s: String) -> MainMenuItem { MainMenuItem { text: s, next: None } }

    pub fn from_with_screen(s: String, screen: Screen) -> MainMenuItem { MainMenuItem { text: s, next: Some(screen) } }
}

impl ToString for MainMenuItem { fn to_string(&self) -> String { self.text.clone() } }

impl Default for MakeDeckFocus {fn default() -> Self { Self::Title } }

#[derive(Default)]
pub struct MakeDeckContents {
    pub focus: MakeDeckFocus,
    pub title: String,
    pub commander: String,
    pub commander2: String,
    pub commander_names: Vec<String>,
}
pub struct MakeDeckScreen<'a> {
    pub title_entry: Paragraph<'a>,
    pub commander_entry: Paragraph<'a>,
    pub commander2_entry: Paragraph<'a>,
    pub potential_commanders: StatefulList<String>,
}

impl<'a> MakeDeckScreen<'a> {
    pub fn new(mdc: &MakeDeckContents) -> MakeDeckScreen<'a> {
        let (te, ce, ce2) = match mdc.focus {
            MakeDeckFocus::Title => {
                (Paragraph::new(mdc.title.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Deck Name")
                        .style(Style::default().fg(Color::Yellow))),
                Paragraph::new(mdc.commander.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Commander")),
                Paragraph::new(""))
            }
            MakeDeckFocus::Commander => {
                (Paragraph::new(mdc.title.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Deck Name")
                        .style(Style::default().fg(Color::Cyan))),
                Paragraph::new(mdc.commander.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Commander"))
                        .style(Style::default().fg(Color::Yellow)), 
                Paragraph::new(""))
            }
            MakeDeckFocus::SecondaryCommander => {
                (Paragraph::new(mdc.title.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Deck Name")
                        .style(Style::default().fg(Color::Cyan))),
                Paragraph::new(mdc.commander.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Commander"))
                        .style(Style::default().fg(Color::Cyan)), 
                Paragraph::new(mdc.commander2.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Second Commander"))
                        .style(Style::default().fg(Color::Yellow)))
            }
        };

        let list = StatefulList::with_items(mdc.commander_names.clone());

        MakeDeckScreen {
            title_entry: te,
            commander_entry: ce,
            commander2_entry: ce2,
            potential_commanders: list
        }
    }
}

pub struct DeckStatScreen<'a> {
    pub mana_curve: BarChart<'a>,
    pub prices: Table<'a>,
    pub type_breakdown: BarChart<'a>,
    pub tag_list: List<'a>,
}

#[derive(Default, Clone)]
pub struct DeckStatInfo {
    pub cmc_data: Vec<(String, u64)>,
    pub price_data: Vec<(String, f64)>,
    pub type_data: Vec<(String, u64)>,
    pub tag_data: Vec<(String, u64)>,
}

impl<'a> DeckStatScreen<'a> {
    pub fn from(
        cmc_data: &'a Vec<(&'a str, u64)>, 
        price_data: &'a Vec<(String, f64)>, 
        type_data: &'a Vec<(&'a str, u64)>,
        tag_data: Vec<ListItem<'a>>) -> DeckStatScreen<'a> {
        

        let mana_curve = BarChart::default()
            .block(Block::default().title("Converted Mana Costs").borders(Borders::ALL))
            .bar_width(3)
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::White).bg(Color::Black))
            .value_style(Style::default().fg(Color::Black).add_modifier(Modifier::BOLD))
            .label_style(Style::default().fg(Color::Cyan))
            .data(cmc_data.as_slice());

        let type_breakdown = BarChart::default()
            .block(Block::default().title("Type Breakdown").borders(Borders::ALL))
            .bar_width(3)
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::White))
            .value_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .label_style(Style::default().fg(Color::Cyan))
            .data(type_data.as_slice());

        let mut prices = Vec::new();
        let mut total = 0.0;
        for (n, v) in price_data {
            total += v;
            let r = Row::new(vec![Cell::from(n.as_str()), Cell::from(v.to_string())]);
            prices.push(r);
        }
        prices.insert(0, Row::new(vec![Cell::from("Total"), Cell::from(total.to_string())])
            .style(Style::default().add_modifier(Modifier::BOLD)));

        let prices = Table::new(prices)
            .style(Style::default().fg(Color::White))
            .header(
                Row::new(vec!["Card", "Price"])
                    .style(Style::default().fg(Color::Yellow)))
            .block(Block::default().title("Card Prices").borders(Borders::ALL))
            .widths(&[Constraint::Length(20), Constraint::Length(6)])
            .column_spacing(1);

        let tag_list = List::new(tag_data)
            .block(Block::default().title("List").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");

        DeckStatScreen {
            mana_curve,
            prices,
            type_breakdown,
            tag_list,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Card {
    pub cmc: f64,
    pub color_identity: Vec<char>,
    // pub legalities: Legalities,
    pub loyalty: String,
    pub mana_cost: String,
    pub name: String,
    pub power: String,
    pub tags: Vec<String>,
    pub text: String,
    pub toughness: String,
    pub types: String,
    pub lo: CardLayout,
    pub rarity: String,
    pub price: Option<f64>,
    pub stale: bool,
    //TODO: Add sets?
}

impl ToString for Card { fn to_string(& self) -> String { self.name.clone() } }
impl Card {
    pub fn display(&self) -> Paragraph {
        let mut v = Vec::new();
        v.push(Spans::from(self.name.clone()));
        v.push(Spans::from(format!("{}, ({})", self.mana_cost, self.cmc)));
        v.push(Spans::from(self.types.clone()));
        v.push(Spans::from(self.rarity.clone()));
        
        v.push(Spans::from(String::new()));
        let t = self.text.split("\n");
        for l in t {
            v.push(Spans::from(l));
        }
        
        let s = if self.power.len() > 0 {
            v.push(Spans::from(String::new()));
            format!("Power/Toughness: {}/{}", self.power, self.toughness)
        } else if self.loyalty.len() > 0 {
            v.push(Spans::from(String::new()));
            format!("Loyalty: {}", self.loyalty.clone())
        } else {
            String::new()
        };
        if s != String::new() {
            v.push(Spans::from(s));
        }
        
        let s = match &self.lo {
            CardLayout::Adventure(side, rel) => { 
                v.push(Spans::from(String::new()));
                match side { 
                    'a' => { format!("Also has Adventure: {}", rel) } 
                    'b' => { format!("Adventure of: {}", rel) } 
                    _ => { String::new() } 
                }
            }
            CardLayout::Aftermath(side, rel) => { 
                v.push(Spans::from(String::new()));
                match side { 
                    'a' => { format!("Also has Aftermath: {}", rel) } 
                    'b' => { format!("Aftermath of: {}", rel) } 
                    _ => { String::new() } 
                }
            }
            CardLayout::Flip(side, rel) => { 
                v.push(Spans::from(String::new()));
                match side { 
                    'a' => { format!("Also has Flip side: {}", rel) } 
                    'b' => { format!("Flip side of: {}", rel) } 
                    _ => { String::new() } 
                }
            }
            CardLayout::ModalDfc(_, rel) => { 
                v.push(Spans::from(String::new()));
                format!("You may instead cast: {}", rel) 
            }
            CardLayout::Split(_, rel) => { 
                v.push(Spans::from(String::new()));
                format!("You may instead cast: {}", rel) 
            }
            CardLayout::Transform(side, rel) => { 
                v.push(Spans::from(String::new()));
                match side { 
                    'a' => { 
                        format!("Transforms into: {}", rel) 
                    } 
                    'b' => { 
                        format!("Transforms from: {}", rel) 
                    } 
                    _ => { String::new() } 
                }
            }
            CardLayout::Meld(side, face, meld) => { 
                v.push(Spans::from(String::new()));
                match side { 
                    'a' => { 
                        format!("Melds with {} to form {}", face, meld) 
                    } 
                    'b' => { 
                        format!("Melds from {} and {}", face, meld) 
                    } 
                    _ => { String::new() } 
                }
            }
            _ => { String::new() }
        };
        if s != String::new() {
            v.push(Spans::from(s));
        }

        v.push(Spans::from(String::new()));
        if let Some(p) = &self.price {
            let style = if self.stale {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            let a = Span::styled(format!("Price: ${}", p), style);
            v.push(Spans::from(a));
        }
        
        if self.tags.len() > 0 {
            v.push(Spans::from(format!("Tags: {}", self.tags.join(" "))));
        }

        Paragraph::new(v)
    }

    pub fn is_commander(&self) -> CommanderType {
        if (self.types.contains("Legendary") && self.types.contains("Creature"))
        || (self.types.contains("Planeswalker") && self.text.contains("can be your commander")) {
            let re = Regex::new(r"Partner with ([\w, ]+)(?:\n| \()").unwrap();
            if let Some(cap) = re.captures(self.text.as_str()) { 
                return CommanderType::PartnerWith((&cap[1]).to_string()) 
            }
            if self.text.contains("Partner") { return CommanderType::Partner }
            return CommanderType::Default
        }
        CommanderType::Invalid
    }
}

#[derive(Clone)]
pub struct CardStat {
    pub cmc: u8,
    pub color_identity: Vec<String>,
    pub mana_cost: String,
    pub name: String,
    pub tags: Vec<String>,
    pub types: String,
    pub price: f64,
}

#[derive(Clone)]
pub struct Deck {
    pub name: String,
    pub commander: Card,
    pub commander2: Option<Card>,
    pub color: String,
    pub id: i32,
}

impl ToString for Deck { fn to_string(& self) -> String { self.name.clone() } }

impl DeckView {
    pub fn new(did: i32, conn: &Connection, vt: Vec<String>, default_filter: DefaultFilter, sort_order: SortOrder) -> DeckView {
        let deck = crate::db::rdfdid(conn, did).unwrap();
        let cf = CardFilter::from(did, &deck.color, default_filter, sort_order);
        let vcdec = rvcnfcf(conn, &cf.make_query(false, "")).unwrap();
        let st = vt.iter().position(|s| s == &String::from("main")).unwrap();
        let mut ls = ListState::default();
        ls.select(Some(0));
        let ac = Some(crate::db::rcfn(conn, &vcdec[0], Some(cf.did)).unwrap());

        DeckView { 
            omni: String::new(), 
            omniprev: String::new(), 
            omnipos: 0, 
            vsomni: Vec::new(), 
            vcde: vcdec.clone(), 
            vcdec, 
            vcdb: Vec::new(), 
            vt,
            st, 
            ac, 
            vcdels: ls, 
            vcdbls: ListState::default(), 
            cf,
            dvs: DeckViewSection::DeckOmni,
        }
    }

    pub fn handle_input(&mut self, c: KeyCode, conn: &Connection) -> DeckViewExit {
        match self.dvs {
            DeckViewSection::DeckOmni | DeckViewSection::DbOmni => {
                match c {
                    KeyCode::Left => {
                        if self.omnipos > 0 {
                            self.omnipos -= 1;
                        }
                    },
                    KeyCode::Right => {
                        if self.omnipos < self.omni.len() {
                            self.omnipos += 1;
                        }
                    },
                    KeyCode::Up => {
                        if let Some(i) = self.vsomni.iter().position(|s| s == &self.omni) {
                            if i > 0 {
                                self.omni = self.vsomni[i-1].clone();
                                self.omnipos = self.omni.len();
                                if self.dvs == DeckViewSection::DeckOmni { self.uvc(conn); }
                            }
                        } else {
                            if let Some(s) = self.vsomni.last() {
                                self.omni = s.clone();
                                self.omnipos = self.omni.len();
                                if self.dvs == DeckViewSection::DeckOmni { self.uvc(conn); }
                            }
                        }
                    },
                    KeyCode::Down => {
                        if let Some(i) = self.vsomni.iter().position(|s| s == &self.omni) {
                            if i < self.vsomni.len() - 1 {
                                self.omni = self.vsomni[i+1].clone();
                                self.omnipos = self.omni.len();
                                if self.dvs == DeckViewSection::DeckOmni { self.uvc(conn); }
                            } else {
                                self.omni = String::new();
                                self.omnipos = 0;
                                if self.dvs == DeckViewSection::DeckOmni { self.uvc(conn); }
                            }
                        }
                    },
                    KeyCode::Home => self.omnipos = 0,
                    KeyCode::End => self.omnipos = self.omni.len(),
                    KeyCode::Delete => {
                        if self.omnipos < self.omni.len() {
                            self.omni.remove(self.omnipos);
                        }
                        if self.dvs == DeckViewSection::DeckOmni {
                            self.uvc(conn);
                        }
                    },
                    KeyCode::Backspace => {
                        if self.omnipos > 0 {
                            self.omni.remove(self.omnipos - 1);
                            self.omnipos -= 1;
                        }
                        if self.dvs == DeckViewSection::DeckOmni {
                            self.uvc(conn);
                        }
                    },
                    KeyCode::Enter => {
                        let so = self.omni.trim();
                        if so == "/stat" {
                            return DeckViewExit::Stats;
                        } else if so == "/settings" || so == "/config" {
                            return DeckViewExit::Settings(self.cf.did);
                        } else {
                            let mut tag = String::new();
                            let re = Regex::new(r"/tag:(\w*)").unwrap();
                            let omni = if let Some(cap) = re.captures(so) { 
                                tag = String::from(&cap[1]);
                                let s = format!("/tag:{}", tag);
                                let s = so.replace(&s, "");
                                let s = s.replace("  ", " ");
                                self.insert_tag(tag.clone());
                                s
                            } else {
                                so.into()
                            };
                            self.omni = omni.clone();

                            if omni.len() > 0 {
                                if let Some(i) = self.vsomni.iter().position(|s| s == &omni) {
                                    self.vsomni.remove(i);
                                };
                                self.vsomni.push(omni.clone());
                            }

                            if self.dvs == DeckViewSection::DbOmni {
                                self.uvc(conn);
                                if self.vcdb.len() > 0 {
                                    self.dvs = DeckViewSection::DbCards;
                                }
                            } else if self.vcde.len() > 0 {
                                self.dvs = DeckViewSection::DeckCards;
                            }

                            if tag.len() > 0 {
                                return DeckViewExit::NewTag(tag, self.cf.did)
                            }
                        }
                    },
                    KeyCode::Esc => {
                        return DeckViewExit::MainMenu
                    },
                    KeyCode::Tab => {
                        (self.omni, self.omniprev) = (self.omniprev.clone(), self.omni.clone());
                        self.omnipos = 0;
                        if self.dvs == DeckViewSection::DbOmni {
                            self.uac(conn);
                            self.dvs =  DeckViewSection::DeckOmni
                        } else {
                            self.uac(conn);
                            self.dvs = DeckViewSection::DbOmni
                        }
                    },
                    KeyCode::Char(c) => {
                        self.omni.insert(self.omnipos, c);
                        self.omnipos += 1;
                        if self.dvs == DeckViewSection::DeckOmni {
                            self.uvc(conn);
                        }
                    },
                    _ => {}
                }
            },
            DeckViewSection::DeckCards | DeckViewSection::DbCards => {
                let (vcd, vcdls) = match self.dvs {
                    DeckViewSection::DeckCards => (&mut self.vcde, &mut self.vcdels),
                    DeckViewSection::DbCards => (&mut self.vcdb, &mut self.vcdbls),
                    _ => todo!(),
                };
                
                match c {
                    KeyCode::Up => {
                        let i = match vcdls.selected() {
                            Some(i) => {
                                if i == 0 {
                                    vcd.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        vcdls.select(Some(i));
                        self.uac(&conn);
                    },
                    KeyCode::Down => {
                        let i = match vcdls.selected() {
                            Some(i) => {
                                if i >= vcd.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        vcdls.select(Some(i));

                        self.uac(&conn);
                    },
                    KeyCode::Right => {
                        self.st += 1;
                        if self.st >= self.vt.len() { self.st = 0; }
                    },
                    KeyCode::Left => {
                        if self.st > 0 { self.st -= 1; }
                        else { self.st = self.vt.len() - 1; }
                    },
                    KeyCode::Delete => {
                        let i = vcdls.selected().unwrap();
                        let cn = &vcd.get(i).unwrap().clone();
                        let c = rcfn(&conn, cn, None).unwrap();
                        let d = crate::db::rdfdid(conn, self.cf.did).unwrap();
                        if d.commander == c { return DeckViewExit::Hold; }
                        if let Some(com) = d.commander2 {
                            if com == c { return DeckViewExit::Hold; }
                        };

                        if let Some(j) = self.vcdec.iter().position(|s| s == cn) {
                            self.vcdec.remove(j);
                            let flag = self.dvs == DeckViewSection::DeckCards;
                            dcntodc(conn, cn, self.cf.did).unwrap();
                            
                            if flag { vcd.remove(i); }
                            match &c.lo {
                                CardLayout::Flip(_, n) | 
                                CardLayout::Split(_, n) | 
                                CardLayout::ModalDfc(_, n) | 
                                CardLayout::Aftermath(_, n) | 
                                CardLayout::Adventure(_, n) | 
                                CardLayout::Transform(_, n) => { 
                                    if flag { 
                                        let pos = vcd.iter().position(| s| s == n).unwrap();
                                        vcd.remove(pos);
                                        if i > pos { vcdls.select(Some(i-1)); }
                                    }
                                    dcntodc(conn, &n, self.cf.did).unwrap();
                                }
                                CardLayout::Meld(s, n, m) => { 
                                    if flag {
                                        let opos = vcd.iter().position(| s| s == m);
                                        if let Some(pos) = opos {
                                            vcd.remove(pos);
                                            if i > pos { vcdls.select(Some(i-1)); }
                                        };
                                    }
                                    let _a = dcntodc(conn, &m, self.cf.did);
    
                                    if s == &'b' { 
                                        if flag {
                                            let pos = vcd.iter().position(| s| s == n).unwrap();
                                            vcd.remove(pos);
                                            if i > pos { vcdls.select(Some(i-1)); }
                                        }
                                        dcntodc(conn, &n, self.cf.did).unwrap();
                                    } 
                                }
                                _ => {}
                            }

                            self.vcdec = rvcnfcf(conn, &self.cf.make_query(false, "")).unwrap();

                            if vcd.len() == 0 { //Can only happen in Deck view
                                vcdls.select(None);
                                self.ac = None;
                                self.dvs = DeckViewSection::DeckOmni;
                            }

                            if !flag {                        
                                let vc = rvcnfcf(&conn, &self.cf.make_query(false, &self.omniprev)).unwrap();
                                if vc.len() > 0 {
                                    self.vcdels.select(Some(0));

                                } else {
                                    self.vcdels.select(None);
                                }
                                self.vcde = vc;
                            } else {
                                self.uac(conn);
                            }
                        }
                    },
                    KeyCode::Enter => {
                        let cn = &self.ac.as_ref().unwrap().name;
                        if self.vcdec.contains(cn) {
                            self.toggle_tag(conn)
                        } else {
                            if let Ok(vc) = crate::db::ictodc(conn, &self.ac.as_ref().unwrap(), self.cf.did) {
                                for c in vc {
                                    self.vcdec.push(c.name);
                                }
                            }
                            let vc = rvcnfcf(&conn, &self.cf.make_query(false, &self.omniprev)).unwrap();
                            if vc.len() > 0 {
                                self.vcdels.select(Some(0));
                            } else {
                                self.vcdels.select(None);
                            }
                            self.vcde = vc;
                        }
                    },
                    KeyCode::Esc => {
                        return DeckViewExit::MainMenu
                    },
                    KeyCode::Tab => {
                        if self.dvs == DeckViewSection::DeckCards {
                            self.dvs = DeckViewSection::DeckOmni;
                        } else {
                            self.dvs = DeckViewSection::DbOmni;
                        }
                    }
                    KeyCode::Char(' ') => self.uacr(conn),
                    KeyCode::Char('u') => {
                        if let Some(ac) = &self.ac {
                            if ac.stale {
                                if let Ok(card) = crate::db::upfcn_detailed(conn, &ac, Some(self.cf.did)) {
                                    self.ac = Some(card);
                                }
                            }
                        };
                    },
                    _ => {}
                }
            }
        }
        DeckViewExit::Hold
    }

    fn insert_tag(&mut self, tag: String) {
        if !self.vt.contains(&tag) {
            match self.vt.iter().position(| s| s > &tag) {
                Some(i) => {
                    self.st = i;
                    self.vt.insert(i, tag);
                },
                None => {
                    self.st = self.vt.len();
                    self.vt.push(tag);
                }
            }
        } else {
            self.st = self.vt.iter().position(|s| s == &tag).unwrap();
        }
    }

    fn toggle_tag(&mut self, conn: &Connection) {
        let cn = self.ac.as_ref().unwrap().to_string();
        self.ac = ttindc(conn, &cn, &self.vt[self.st], self.cf.did);
    }

    fn uvc(&mut self, conn: &Connection) {
        let (vc, general, ls) = match self.dvs {
            DeckViewSection::DeckOmni | DeckViewSection::DeckCards => (&mut self.vcde, false, &mut self.vcdels),
            DeckViewSection::DbOmni | DeckViewSection::DbCards => (&mut self.vcdb, true, &mut self.vcdbls)
        };

        *vc = rvcnfcf(&conn, &self.cf.make_query(general, &self.omni)).unwrap();
        if vc.len() > 0 {
            ls.select(Some(0));

        } else {
            ls.select(None);
        }
        self.uac(conn);
    }
    
    // fn uacfn(&mut self, conn: &Connection, cn: &String) {
    //     self.ac = Some(crate::db::rcfndid(conn, cn, self.cf.did).unwrap());
    // }

    fn uac(&mut self, conn: &Connection) {
        let mm = String::new(); //this is dumb, but it works. Otherwise complains of temp value dropped.
        let cn = match self.dvs {
            // Screen::DeckView(dvs) => {
            //     if self.vcde.len() == 0 { &mm 
            //     } else {
            //         match dvs {
            //             DeckViewSection::Omni => &self.vcde[0],
            //             DeckViewSection::Cards => &self.vcde[self.vcdels.selected().unwrap()],
            //         }
            //     }
            // },
            // Screen::DatabaseView(dvs) => {
            //     if self.vcdb.len() == 0 { &mm 
            //     } else {
            //         match dvs {
            //             DeckViewSection::Omni => &self.vcdb[0],
            //             DeckViewSection::Cards => &self.vcdb[self.vcdbls.selected().unwrap()],
            //         }
            //     }
            // },
            // _ => { &mm }
            DeckViewSection::DeckOmni => self.vcde.get(0).unwrap_or(&mm),
            DeckViewSection::DeckCards => self.vcde.get(self.vcdels.selected().unwrap()).unwrap_or(&mm),
            DeckViewSection::DbOmni => self.vcdb.get(0).unwrap_or(&mm),
            DeckViewSection::DbCards => self.vcdb.get(self.vcdbls.selected().unwrap()).unwrap_or(&mm),
        };
        if cn == &mm { 
            self.ac = None; 
        } else { 
            self.ac = Some(crate::db::rcfn(conn, cn, Some(self.cf.did)).unwrap()); 
        }
    }

    fn uacr(&mut self, conn: &Connection) {
        let c = self.ac.as_ref().unwrap();
        let cn = match &c.lo {
            crate::util::CardLayout::Flip(_, n) | 
            crate::util::CardLayout::Split(_, n) | 
            crate::util::CardLayout::ModalDfc(_, n) | 
            crate::util::CardLayout::Aftermath(_, n) | 
            crate::util::CardLayout::Adventure(_, n) | 
            crate::util::CardLayout::Transform(_, n) => { n },
            crate::util::CardLayout::Meld(s, n, m) => { 
                // side = s; rel = n.clone(); rel2 = m.clone(); 
                if s == &'b' { n }
                else { 
                    let meld = rcfn(conn, &m, None).unwrap();
                    if let crate::util::CardLayout::Meld(_, face, _) = meld.lo {
                        if &face == n {
                            m
                        } else {
                            n
                        }
                    } else {
                        n //Should never occur, but need to complete if statement
                    }
                }
            },
            _ => { return; }
        };

        self.ac = Some(crate::db::rcfn(conn, cn, Some(self.cf.did)).unwrap());
    }
    
    pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
        let mut vrct = Vec::new();
        let cut = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
            .split(frame.size());

        vrct.append(&mut Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Length(18)].as_ref())
            .split(cut[0]));
        
        vrct.append(&mut Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(26),Constraint::Min(18)].as_ref())
            .split(cut[1]));

        let mut bdef = Block::default().borders(Borders::ALL);
        let mut bfoc = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let spans = if self.omnipos < self.omni.len() {
            let (s1, s2) = self.omni.split_at(self.omnipos);
            let (s2, s3) = s2.split_at(1);
            vec![
                Span::styled(s1, Style::default()),
                Span::styled(s2, Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::styled(s3, Style::default()),
            ]
        } else {
            vec![
                Span::styled(self.omni.as_str(), Style::default()),
                Span::styled(" ", Style::default().add_modifier(Modifier::UNDERLINED)),
            ]
        };

        let mut po = Paragraph::new(Spans::from(spans));
        let pt = Paragraph::new(self.vt[self.st].clone()).block(bdef.clone());
        let pc = match &self.ac {
            Some(card) => card.display().block(bdef.clone()),
            None => Paragraph::new("No card found!").block(bdef.clone()),
        };

        let (lc, ls) = match self.dvs {
            DeckViewSection::DeckOmni => {
                bfoc = bfoc.title("Filter Deck");
                po = po.block(bfoc);
                let vli: Vec<ListItem> = self.vcde.iter().map(|s| ListItem::new(s as &str)).collect();
                let lc = List::new(vli)
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
                    .block(bdef.title(format!("Deck View ({})", self.vcde.len())));
                (lc, &self.vcdels)
            },
            DeckViewSection::DeckCards => {
                bdef = bdef.title("Filter Deck");
                po = po.block(bdef);
                let vli: Vec<ListItem> = self.vcde.iter().map(|s| ListItem::new(s as &str)).collect();
                let lc = List::new(vli)
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
                    .block(bfoc.title(format!("Deck View ({})", self.vcde.len())));
                (lc, &self.vcdels)
            },
            DeckViewSection::DbOmni => {
                bfoc = bfoc.title("Filter Database");
                po = po.block(bfoc);
                let vli: Vec<ListItem> = self.vcdb.iter().map(|s|
                        if self.vcdec.contains(s) {
                            ListItem::new(s as &str).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))
                        } else {
                            ListItem::new(s as &str)
                        }).collect();
                let lc = List::new(vli)
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
                    .block(bdef.title(format!("Database ({})", self.vcdb.len())));
                (lc, &self.vcdbls)
            },
            DeckViewSection::DbCards => {
                bdef = bdef.title("Filter Database");
                po = po.block(bdef);
                let vli: Vec<ListItem> = self.vcdb.iter().map(|s|
                        if self.vcdec.contains(s) {
                            ListItem::new(s as &str).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))
                        } else {
                            ListItem::new(s as &str)
                        }).collect();
                let lc = List::new(vli)
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
                    .block(bfoc.title(format!("Database ({})", self.vcdb.len())));
                (lc, &self.vcdbls)
            }
        };

        frame.render_widget(po, vrct[0]);
        frame.render_widget(pt, vrct[1]);
        frame.render_stateful_widget(lc, vrct[2], &mut ls.clone());
        frame.render_widget(pc, vrct[3]);
    }
}

pub mod views {
    use crossterm::event::KeyCode;
    use tui::{backend::CrosstermBackend, layout::{Layout, Direction, Constraint, Alignment, Rect}, widgets::{Paragraph, Block, Borders}, text::{Span, Spans}, style::{Style, Modifier, Color}};

    use super::{DefaultFilter, SortOrder};

    #[derive(Copy, Clone, PartialEq)]
    pub enum SettingsSection {
        Tags,
        TagText,
        DefaultFilter,
        Ordering,
        OpenIntoRecent,
        Save,
        Exit,
    }
    
    pub enum SettingsExit {
        Save(Changes),
        Hold,
        Cancel
    }

    #[derive(PartialEq, Clone)]
    pub enum TagChange {
        DeleteTag(String),
        ChangeTag(String, String),
        InsertTag(String),
    }

    pub struct Changes {
        pub df: DefaultFilter,
        pub so: SortOrder,
        pub oir: Option<bool>,
        pub vtch: Vec<TagChange>,
    }

    pub struct SettingsView {
        section: SettingsSection,
        title: String,
        vt: Vec<String>,
        wt: String,
        vpos: usize,
        tpos: usize,
        df: DefaultFilter,
        ord: SortOrder,
        oir: Option<bool>,
        vch: Vec<TagChange>,
    }


    impl SettingsView {
        pub fn new(
            mut vt: Vec<String>, 
            df: DefaultFilter, 
            ord: SortOrder, 
            n: String, 
            oir: Option<bool>) -> SettingsView {
            vt.sort();
            vt.push(String::from("{Add new tag}"));

            SettingsView {
                section: SettingsSection::Tags,
                title: n,
                vt,
                wt: String::new(),
                vpos: 0,
                tpos: 0,
                df,
                ord,
                oir,
                vch: Vec::new(),
            }
        }

        pub fn handle_input(&mut self, c: KeyCode) -> SettingsExit {
            match c {
                KeyCode::Esc => SettingsExit::Cancel,
                KeyCode::Right => {
                    match self.section {
                        SettingsSection::Tags => self.vpos = (self.vpos + 1) % self.vt.len(),
                        SettingsSection::TagText => {
                            let max = self.vt[self.vpos].len();
                            if self.tpos < max {
                                self.tpos += 1;
                            }
                        },
                        SettingsSection::DefaultFilter => {
                            self.df = match self.df {
                                DefaultFilter::Name => DefaultFilter::Text,
                                DefaultFilter::Text => DefaultFilter::Name,
                            };
                        },
                        SettingsSection::Ordering => {
                            self.ord = match self.ord {
                                SortOrder::NameAsc => SortOrder::NameDesc,
                                SortOrder::NameDesc => SortOrder::CmcAsc,
                                SortOrder::CmcAsc => SortOrder::CmcDesc,
                                SortOrder::CmcDesc => SortOrder::NameAsc,
                            }
                        },
                        SettingsSection::OpenIntoRecent => {
                            if let Some(f) = self.oir {
                                self.oir = Some(!f);
                            };
                        },
                        SettingsSection::Save => self.section = SettingsSection::Exit,
                        SettingsSection::Exit => self.section = SettingsSection::Save,
                    };
                    SettingsExit::Hold
                },
                KeyCode::Left => {
                    match self.section {
                        SettingsSection::Tags => { 
                            if self.vpos == 0 {
                                self.vpos = self.vt.len() - 1;
                            } else {
                                self.vpos -= 1;
                            }
                        },
                        SettingsSection::TagText => {
                            if self.tpos > 0 {
                                self.tpos -= 1;
                            }
                        },
                        SettingsSection::DefaultFilter => {
                            self.df = match self.df {
                                DefaultFilter::Name => DefaultFilter::Text,
                                DefaultFilter::Text => DefaultFilter::Name,
                            };
                        },
                        SettingsSection::Ordering => {
                            self.ord = match self.ord {
                                SortOrder::NameAsc => SortOrder::NameDesc,
                                SortOrder::NameDesc => SortOrder::CmcAsc,
                                SortOrder::CmcAsc => SortOrder::CmcDesc,
                                SortOrder::CmcDesc => SortOrder::NameAsc,
                            };
                        },
                        SettingsSection::OpenIntoRecent => {
                            if let Some(f) = self.oir {
                                self.oir = Some(!f);
                            };
                        },
                        SettingsSection::Save => self.section = SettingsSection::Exit,
                        SettingsSection::Exit => self.section = SettingsSection::Save,
                    };
                    SettingsExit::Hold
                },
                KeyCode::Down => {
                    self.section = match self.section {
                        SettingsSection::Tags => SettingsSection::DefaultFilter,
                        SettingsSection::TagText => SettingsSection::TagText,
                        SettingsSection::DefaultFilter => SettingsSection::Ordering,
                        SettingsSection::Ordering => {
                            if self.oir == None { SettingsSection::Save } 
                            else { SettingsSection::OpenIntoRecent }
                        },
                        SettingsSection::OpenIntoRecent => SettingsSection::Save,
                        SettingsSection::Save => SettingsSection::Tags,
                        SettingsSection::Exit => SettingsSection::Tags,
                    };
                    SettingsExit::Hold
                },
                KeyCode::Up => {
                    self.section = match self.section {
                        SettingsSection::Tags => SettingsSection::Save,
                        SettingsSection::TagText => SettingsSection::TagText,
                        SettingsSection::DefaultFilter => SettingsSection::Tags,
                        SettingsSection::Ordering => SettingsSection::DefaultFilter,
                        SettingsSection::OpenIntoRecent => SettingsSection::Ordering,
                        SettingsSection::Save => {
                            if self.oir == None { SettingsSection::Ordering } 
                            else { SettingsSection::OpenIntoRecent }
                        },
                        SettingsSection::Exit => {
                            if self.oir == None { SettingsSection::Ordering } 
                            else { SettingsSection::OpenIntoRecent }
                        },
                    };
                    SettingsExit::Hold
                },
                KeyCode::Enter => {
                    match self.section {
                        SettingsSection::Tags => {
                            if self.vt[self.vpos] != String::from("main") 
                            || self.vt.iter().filter(|&s| s == &String::from("main")).count() > 1 {
                                self.section = SettingsSection::TagText;
                                if self.vpos == self.vt.len() - 1 {
                                    self.vt[self.vpos] = String::new();
                                    self.wt = String::new();
                                } else {
                                    self.wt = self.vt[self.vpos].clone();
                                }
                            }
                            SettingsExit::Hold
                        },
                        SettingsSection::TagText => {
                            self.tpos = 0;
                            self.section = SettingsSection::Tags;
                            if self.vt[self.vpos].len() == 0 {
                                self.vt.remove(self.vpos);
                                self.vch.push(TagChange::DeleteTag(self.wt.clone()));
                            }
                            if self.wt.is_empty() {
                                self.vch.push(TagChange::InsertTag(self.vt[self.vpos].clone()));
                                self.vt.sort();
                                self.vt.push(String::from("{Add new tag}"));
                            } else {
                                self.vch.push(TagChange::ChangeTag(self.wt.clone(), self.vt[self.vpos].clone()));
                                let s = self.vt.pop().unwrap();
                                self.vt.sort();
                                self.vt.push(s.clone());
                            }
                            SettingsExit::Hold
                        },
                        SettingsSection::DefaultFilter => SettingsExit::Hold,
                        SettingsSection::Ordering => SettingsExit::Hold,
                        SettingsSection::OpenIntoRecent => SettingsExit::Hold,
                        SettingsSection::Save => {
                            let changes = Changes { df: self.df, so: self.ord, oir: self.oir, vtch: self.vch.clone() };
                            SettingsExit::Save(changes)
                        },
                        SettingsSection::Exit => SettingsExit::Cancel,
                    }
                },
                KeyCode::Delete => {
                    match self.section {
                        SettingsSection::Tags => {
                            if self.vpos < self.vt.len() - 1
                            && (self.vt[self.vpos] != String::from("main") 
                            || self.vt.iter().filter(|&s| s == &String::from("main")).count() > 1) {
                                self.vch.push(TagChange::DeleteTag(self.vt[self.vpos].clone()));
                                self.vt.remove(self.vpos);
                            }
                        },
                        SettingsSection::TagText => {
                            let s = &mut self.vt[self.vpos];
                            if self.tpos < s.len() {
                                s.remove(self.tpos);
                            }
                        },
                        _ => {}
                    }
                    SettingsExit::Hold
                },
                KeyCode::Backspace => {
                    if self.tpos > 0 {
                        self.vt[self.vpos].remove(self.tpos - 1);
                        self.tpos -= 1;
                    }
                    SettingsExit::Hold
                },
                KeyCode::Char(c) => {
                    if self.section == SettingsSection::TagText {
                            if self.vt[self.vpos] != String::from("main") 
                            || self.vt.iter().filter(|&s| s == &String::from("main")).count() > 1 {
                                self.vt[self.vpos].insert(self.tpos, c);
                                self.tpos += 1;
                            }
                    }
                    SettingsExit::Hold
                },
                _ => SettingsExit::Hold,
            }
        }

        pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
            let mut st = self.vt.get(self.vpos).unwrap().clone();
            let mut vsp = Vec::new();
            for i in 0..self.vt.len() {
                let mut s = self.vt[i].clone();
                s.push(' ');
                if i == self.vpos {
                    let span = Span::styled(s, Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
                    vsp.push(Span::from(span));
                } else {
                    vsp.push(Span::from(s));
                }
            }
            if self.section == SettingsSection::TagText {
                vsp.remove(self.vpos);
                st.push(' ');
                let (s1, s2) = st.split_at(self.tpos);
                let (s2, s3) = s2.split_at(1);
                let vs = vec![
                    // Span::from(" "),
                    Span::styled(s3, Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                    Span::styled(s2, Style::default().add_modifier(Modifier::UNDERLINED).add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                    Span::styled(s1, Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                ];

                for s in vs {
                    vsp.insert(self.vpos, s);
                }
            }
            let spans = Spans::from(vsp);
            let length = spans.width();
            let mut ts = Paragraph::new(spans)
                .wrap(tui::widgets::Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title("Tags"));

            let dfpt = match self.df {
                DefaultFilter::Name => "Default filter uses card name",
                DefaultFilter::Text => "Default filter uses card text",
            };
            let mut dfp = Paragraph::new(dfpt)
                .block(Block::default().borders(Borders::ALL).title("Default Filter"));

            let ordt = match self.ord {
                SortOrder::NameAsc => "Cards ordered by name ascending.",
                SortOrder::CmcAsc => "Cards ordered by mana cost ascending.",
                SortOrder::NameDesc => "Cards ordered by name descending.",
                SortOrder::CmcDesc => "Cards ordered by mana cost descending.",
            };
            let mut ordp = Paragraph::new(ordt)
                .block(Block::default().borders(Borders::ALL).title("Default Ordering"));

            let mut oirp = match self.oir {
                Some(true) => Paragraph::new("Open into most recent deck (if it exists)")
                    .block(Block::default().borders(Borders::ALL).title("Quickstart?")),
                Some(false) => Paragraph::new("Open into the main menu")
                    .block(Block::default().borders(Borders::ALL).title("Quickstart?")),
                None => Paragraph::new(""),
            };

            let mut sp = Paragraph::new("Save")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));

            let mut cp = Paragraph::new("Cancel")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
                
            match self.section {
                SettingsSection::Tags => {
                    ts = ts.block(Block::default()
                        .borders(Borders::ALL)
                        .title("Tags")
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                },
                SettingsSection::TagText => {
                    ts = ts.block(Block::default()
                        .borders(Borders::ALL)
                        .title("Tags")
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                },
                SettingsSection::DefaultFilter => {
                    dfp = dfp.block(Block::default()
                        .borders(Borders::ALL)
                        .title("Default Filter")
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                },
                SettingsSection::Ordering => {
                    ordp = ordp.block(Block::default()
                        .borders(Borders::ALL)
                        .title("Default Ordering")
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                    },
                SettingsSection::OpenIntoRecent => {
                    oirp = oirp.block(Block::default()
                        .borders(Borders::ALL)
                        .title("Quickstart?")
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                },
                SettingsSection::Save => {
                    sp = sp.block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                },
                SettingsSection::Exit => {
                    cp = cp.block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default()
                        .fg(Color::Yellow)));
                },
            }

            let theight = 3 + (length as u16 / frame.size().width);

            let mut vrct = Vec::new();
            let constraints = [
                Constraint::Length(theight), 
                Constraint::Length(3), 
                Constraint::Length(3), 
                Constraint::Length(3), 
                // Constraint::Length(3), 
                Constraint::Length(3)];
            let mut cut = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints.as_ref())
                .margin(1)
                .split(frame.size());
            let last = cut.pop().unwrap();
            let buttons = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(10), Constraint::Length(10)].as_ref())
                .margin(1)
                .split(last);
            vrct.append(&mut cut);
            for r in buttons {
                vrct.push(Rect {
                    x: r.x-1,
                    y: r.y-1,
                    width: 10,
                    height: 3,
                })
            }

            frame.render_widget(ts, vrct[0]);
            frame.render_widget(dfp, vrct[1]);
            frame.render_widget(ordp, vrct[2]);
            frame.render_widget(oirp, vrct[3]);
            frame.render_widget(sp, vrct[4]);
            frame.render_widget(cp, vrct[5]);
        }
    }
}