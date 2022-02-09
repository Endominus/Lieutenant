// use crossterm::event::KeyCode;
use regex::Regex;
use rusqlite::Connection;
use tui::style::{Color, Modifier, Style};
use tui::{
    layout::Constraint,
    text::{Span, Spans},
    widgets::{
        BarChart, Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table,
        TableState,
    },
};

// use crate::db::{dcntodc, rcfn, rvcnfcf, ttindc, CardFilter};
use config::{Config, ConfigError};
use itertools::Itertools;
use serde::Deserialize;
use serde_derive::Serialize;
use std::cell::RefCell;
use std::rc::Rc;
// use std::convert::TryInto;
use std::{collections::HashMap, env, path::PathBuf};

use self::views::Changes;

pub fn get_local_file(name: &str, file_must_exist: bool) -> PathBuf {
    let mut p = env::current_exe().unwrap();
    p.pop();
    p.push(name);
    if file_must_exist && !p.exists() {
        panic!(
            "Cannot find the {} file. Are you sure it's in the same directory as the executable?",
            name
        );
    }

    p
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum SortOrder {
    #[default]
    NameAsc,
    NameDesc,
    CmcAsc,
    CmcDesc,
}

#[derive(PartialEq)]
pub enum CommanderType {
    Default,
    Partner,
    PartnerWith(String),
    Invalid,
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum DefaultFilter {
    #[default]
    Name,
    Text,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    MakeDeck,
    OpenDeck,
    Settings,
    DeckView,
    DeckStat,
    // Error(&'static str),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum CardLayout {
    Leveler,
    Meld(char, String, String),
    Paired(char, String, String),
    #[default]
    Normal,
    Saga,
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

// Helper struct so that I don't have to write a custom deserializer
#[derive(Debug, Deserialize, Serialize)]
pub struct FileSettings {
    global: FileGlobalSettings,
    decks: HashMap<i32, FileDeckSettings>,
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

#[derive(Debug)]
pub struct Settings {
    global: GlobalSettings,
    decks: HashMap<i32, Rc<RefCell<DeckSettings>>>,
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
        s.set_default("global.ordering", String::from("+name"))
            .unwrap();
        s.set_default("global.default_filter", String::from("name"))
            .unwrap();
        s.set_default("decks", ds).unwrap();
        s.merge(config::File::with_name(path.to_str().unwrap()))
            .unwrap();

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
        vr.push(format!(
            "open_into_recent = {}",
            self.global.open_into_recent
        ));
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
        match self.decks.get(&did) {
            Some(d) => d.borrow().tags.clone(),
            None => self.global.tags.clone(),
        }
    }

    pub fn rds(&self, did: i32) -> Rc<RefCell<DeckSettings>> {
        let a = self.decks.get(&did).unwrap();
        a.clone()
    }

    pub fn rso(&self, odid: Option<i32>) -> SortOrder {
        match odid {
            Some(did) => match self.decks.get(&did) {
                Some(d) => d.borrow().ordering,
                None => self.global.ordering,
            },
            None => self.global.ordering,
        }
    }

    pub fn rdf(&self, odid: Option<i32>) -> DefaultFilter {
        match odid {
            Some(did) => match self.decks.get(&did) {
                Some(d) => d.borrow().df,
                None => self.global.df,
            },
            None => self.global.df,
        }
    }

    pub fn change(&mut self, changes: &Changes, odid: Option<i32>) {
        match odid {
            Some(did) => {
                if let Some(deck) = self.decks.get_mut(&did) {
                    let mut deck = deck.borrow_mut();
                    deck.df = changes.df;
                    deck.ordering = changes.so;
                    for tch in &changes.vtch {
                        match tch {
                            views::TagChange::DeleteTag(old) => {
                                if let Some(i) = deck.tags.iter().position(|s| s == old) {
                                    deck.tags.remove(i);
                                }
                            }
                            views::TagChange::ChangeTag(old, new) => {
                                if let Some(i) = deck.tags.iter().position(|s| s == old) {
                                    deck.tags.remove(i);
                                    deck.tags.push(new.clone());
                                }
                            }
                            views::TagChange::InsertTag(new) => deck.tags.push(new.clone()),
                        }
                        deck.tags.sort();
                    }
                }
            }
            None => {
                self.global.df = changes.df;
                self.global.ordering = changes.so;
                self.global.open_into_recent = changes.oir.unwrap();
                for tch in &changes.vtch {
                    match tch {
                        views::TagChange::DeleteTag(old) => {
                            if let Some(i) = self.global.tags.iter().position(|s| s == old) {
                                self.global.tags.remove(i);
                            }
                        }
                        views::TagChange::ChangeTag(old, new) => {
                            if let Some(i) = self.global.tags.iter().position(|s| s == old) {
                                self.global.tags.remove(i);
                                self.global.tags.push(new.clone());
                            }
                        }
                        views::TagChange::InsertTag(new) => self.global.tags.push(new.clone()),
                    }
                    self.global.tags.sort();
                }
            }
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
            Some(did) => match self.decks.get_mut(&did) {
                Some(d) => d.borrow_mut().add_tag(tag),
                None => {
                    println!("Invalid Deck ID!");
                }
            },
            None => {
                self.global.tags.push(tag);
                self.global.tags.sort();
            }
        };
    }

    pub fn id(&mut self, did: i32) {
        let ds = DeckSettings::duplicate(&self.global);
        self.decks.insert(did, Rc::from(RefCell::from(ds)));
    }

    pub fn dd(&mut self, deck: i32) {
        self.decks.remove(&deck);
    }

    pub fn from(fs: FileSettings) -> Self {
        let gs = GlobalSettings::from(fs.global);
        let mut dhash = HashMap::new();

        for (did, fds) in fs.decks {
            let ds = DeckSettings::from(fds);
            dhash.insert(did, Rc::from(RefCell::from(ds)));
        }

        Self {
            global: gs,
            decks: dhash,
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
        vr.push(format!(
            "ordering = \"{}\"",
            self.global.ordering.to_string()
        ));
        vr.push(format!(
            "default_filter = \"{}\"",
            self.global.df.to_string()
        ));
        vr.push(format!("recent = {}", self.global.recent));
        vr.push(format!(
            "open_into_recent = {}",
            self.global.open_into_recent
        ));
        vr.push(String::from("\n[decks]"));

        // TODO: Explore using a BTreeMap instead to lose dependence on itertools
        let vk = self.decks.keys().sorted();

        for k in vk {
            let v = self.decks.get(k).unwrap().borrow();
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

    pub fn find_tag(&self, tag: &String) -> Option<usize> {
        self.tags.iter().position(|s| s == tag)
    }

    pub fn toggle_df(&mut self) {
        if self.df == DefaultFilter::Name {
            self.df = DefaultFilter::Text;
        } else {
            self.df = DefaultFilter::Name;
        }
    }
}

#[derive(Default)]
pub struct StatefulList<T: ToString + PartialEq + Clone> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T: ToString + PartialEq + Clone> StatefulList<T> {
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

    pub fn reinitialize(&mut self, items: Vec<T>, restore_pos: bool) {
        let old = match self.state.selected() {
            Some(i) => Some(self.items[i].clone()),
            None => None,
        };
        self.items = items;
        if self.items.len() > 0 {
            if restore_pos && old != None {
                let old = old.unwrap();
                let i = self.items.iter().position(|p| p == &old);
                match i {
                    Some(_) => self.state.select(i),
                    None => self.state.select(Some(0)),
                }
            } else {
                self.state.select(Some(0));
            }
        } else {
            self.state.select(None);
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
        for (i, item) in self.items.iter().enumerate() {
            if item == selected {
                self.state.select(Some(i));
                break;
            }
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn get(&self) -> Option<&T> {
        // There should be a more elegant way of doing this.
        if let Some(s) = self.state.selected() {
            Some(&self.items[s])
        } else {
            None
        }
    }

    pub fn get_string(&self) -> Option<String> {
        if self.items.len() > 0 {
            let a = self.items.get(self.state.selected().unwrap()).unwrap();
            return Some(a.to_string());
        }
        None
    }

    pub fn remove(&mut self) -> T {
        let mut a = self.state.selected().unwrap();
        let removed = self.items.remove(a);
        if self.items.len() > 0 {
            if a == self.items.len() {
                a -= 1;
            }
            self.state.select(Some(a));
        } else {
            self.state.select(None);
        }

        removed
    }

    pub fn remove_named(&mut self, s: &String) {
        let mut i = 999999;
        let a = self.state.selected().unwrap();
        for (j, item) in self.items.iter().enumerate() {
            if &item.to_string() == s {
                i = j
            }
        }
        if i < 999999 {
            self.items.remove(i);
            if i < a {
                self.state.select(Some(a - 1));
            }
        }
    }

    pub fn replace(&mut self, obj: T) {
        let a = self.state.selected().unwrap();
        self.items.remove(a);
        self.items.insert(a, obj);
    }

    pub fn rvli(&self) -> Vec<ListItem> {
        self.items
            .iter()
            .map(|f| ListItem::new(f.to_string()))
            .collect()
    }

    pub fn rvlis(&self, vcn: &Vec<String>) -> Vec<ListItem> {
        self.items
            .iter()
            .map(|f| {
                if vcn.contains(&f.to_string()) {
                    ListItem::new(f.to_string()).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::ITALIC),
                    )
                } else {
                    ListItem::new(f.to_string())
                }
            })
            .collect()
    }
}

#[derive(Default)]
pub struct OpenDeckTable {
    decks: Vec<Deck>,
    pub state: TableState,
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
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        );
        let mut rows = Vec::new();

        for deck in decks {
            let (height, com2) = match deck.commander2 {
                Some(c) => (2, c.name),
                None => (1, String::new()),
            };

            let r = Row::new(vec![
                Cell::from(deck.id.to_string()),
                Cell::from(deck.name),
                Cell::from(format!("{}\n{}", deck.commander.name, com2)),
                Cell::from(deck.color),
            ])
            .height(height)
            .style(Style::default());

            rows.push(r);
        }

        let table = Table::new(rows)
            .header(headers)
            .block(Block::default().borders(Borders::ALL))
            .widths(&[
                Constraint::Length(4),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Length(7),
            ])
            .column_spacing(1)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            );

        table
    }

    pub fn next(&mut self) {
        if self.decks.len() == 0 {
            return;
        }
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
        if self.decks.len() == 0 {
            return;
        }
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
        if self.decks.len() == 0 {
            return None;
        }
        if let Some(i) = self.state.selected() {
            self.decks.get(i)
        } else {
            None
        }
    }

    pub fn remove(&mut self) -> Option<Deck> {
        if self.decks.len() == 0 {
            return None;
        }
        let a = self.state.selected().unwrap();
        let d = self.decks.remove(a);

        if self.decks.len() == 0 {
            self.state = TableState::default();
        } else if self.decks.len() == a {
            self.state.select(Some(a - 1));
        }

        Some(d)
    }
}

#[derive(PartialEq, Clone)]
pub struct MainMenuItem {
    pub text: String,
    pub next: Option<Screen>,
}

impl MainMenuItem {
    pub fn from(s: String) -> MainMenuItem {
        MainMenuItem {
            text: s,
            next: None,
        }
    }

    pub fn from_with_screen(s: String, screen: Screen) -> MainMenuItem {
        MainMenuItem {
            text: s,
            next: Some(screen),
        }
    }
}

impl ToString for MainMenuItem {
    fn to_string(&self) -> String {
        self.text.clone()
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
        tag_data: Vec<ListItem<'a>>,
    ) -> DeckStatScreen<'a> {
        let mana_curve = BarChart::default()
            .block(
                Block::default()
                    .title("Converted Mana Costs")
                    .borders(Borders::ALL),
            )
            .bar_width(3)
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::White).bg(Color::Black))
            .value_style(
                Style::default()
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .label_style(Style::default().fg(Color::Cyan))
            .data(cmc_data.as_slice());

        let type_breakdown = BarChart::default()
            .block(
                Block::default()
                    .title("Type Breakdown")
                    .borders(Borders::ALL),
            )
            .bar_width(3)
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::White))
            .value_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .label_style(Style::default().fg(Color::Cyan))
            .data(type_data.as_slice());

        let mut prices = Vec::new();
        let mut total = 0.0;
        for (n, v) in price_data {
            total += v;
            let r = Row::new(vec![Cell::from(n.as_str()), Cell::from(v.to_string())]);
            prices.push(r);
        }
        prices.insert(
            0,
            Row::new(vec![Cell::from("Total"), Cell::from(total.to_string())])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        );

        let prices = Table::new(prices)
            .style(Style::default().fg(Color::White))
            .header(Row::new(vec!["Card", "Price"]).style(Style::default().fg(Color::Yellow)))
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

impl ToString for Card {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}
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
            CardLayout::Paired(_, message, rel) => {
                v.push(Spans::from(String::new()));
                format!("{message}: {rel}")
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
                    _ => String::new(),
                }
            }
            _ => String::new(),
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

        Paragraph::new(v).wrap(tui::widgets::Wrap { trim: false })
    }

    pub fn is_commander(&self) -> CommanderType {
        if (self.types.contains("Legendary") && self.types.contains("Creature"))
            || (self.types.contains("Planeswalker") && self.text.contains("can be your commander"))
        {
            let re = Regex::new(r"Partner with ([\w, ]+)(?:\n| \()").unwrap();
            if let Some(cap) = re.captures(self.text.as_str()) {
                return CommanderType::PartnerWith((&cap[1]).to_string());
            }
            if self.text.contains("Partner") {
                return CommanderType::Partner;
            }
            return CommanderType::Default;
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

impl ToString for Deck {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

pub mod views {
    use crossterm::event::KeyCode;
    use rusqlite::Connection;
    use std::rc::Rc;
    use std::{
        cell::RefCell,
        convert::TryInto,
        sync::{Arc, Mutex},
    };
    use tui::{
        backend::CrosstermBackend,
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Span, Spans},
        widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    };

    use crate::db::*;

    use super::*;

    fn centered_rect(percent_x: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(r.height / 2 - 3),
                    Constraint::Length(5),
                    Constraint::Length(r.height / 2 - 2),
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

    pub enum ViewExit {
        Save(Changes),
        NewDeck(i32),
        Hold,
        Cancel,
    }

    #[derive(PartialEq, Clone)]
    pub enum TagChange {
        DeleteTag(String),
        ChangeTag(String, String),
        InsertTag(String),
    }

    #[derive(Copy, Clone, PartialEq)]
    enum CreateDeckSection {
        Title,
        PrimaryCommander,
        SecondaryCommander,
    }

    #[derive(Copy, Clone, PartialEq)]
    enum DeckViewSection {
        DeckOmni,
        DeckCards,
        DbOmni,
        DbCards,
    }

    pub enum DeckViewExit {
        Hold,
        MainMenu,
        Stats,
        Settings(i32),
        NewTag(String, i32),
    }

    pub enum OpenDeckViewExit {
        Hold,
        Cancel,
        OpenDeck(i32),
        DeleteDeck(i32),
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

    pub struct DeckView {
        omni: String,
        omniprev: String,
        omnipos: usize,
        vsomni: Vec<String>,
        slde: StatefulList<String>,
        sldb: StatefulList<String>,
        vcdec: Vec<String>,
        st: usize,
        ac: Option<Card>,
        cf: CardFilter,
        dvs: DeckViewSection,
        settings: Rc<RefCell<DeckSettings>>,
        dbc: Arc<Mutex<Connection>>,
    }

    pub struct CreateDeckView {
        section: CreateDeckSection,
        title: String,
        com1: String,
        com2: String,
        vcn: Vec<String>,
        vpos: ListState,
        tpos: usize,
    }

    pub struct OpenDeckView {
        decks: Vec<Deck>,
        state: TableState,
        deleting: bool,
    }

    impl SettingsView {
        pub fn new(
            mut vt: Vec<String>,
            df: DefaultFilter,
            ord: SortOrder,
            n: String,
            oir: Option<bool>,
        ) -> SettingsView {
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

        pub fn handle_input(&mut self, c: KeyCode) -> ViewExit {
            match c {
                KeyCode::Esc => ViewExit::Cancel,
                KeyCode::Right => {
                    match self.section {
                        SettingsSection::Tags => self.vpos = (self.vpos + 1) % self.vt.len(),
                        SettingsSection::TagText => {
                            let max = self.vt[self.vpos].len();
                            if self.tpos < max {
                                self.tpos += 1;
                            }
                        }
                        SettingsSection::DefaultFilter => {
                            self.df = match self.df {
                                DefaultFilter::Name => DefaultFilter::Text,
                                DefaultFilter::Text => DefaultFilter::Name,
                            };
                        }
                        SettingsSection::Ordering => {
                            self.ord = match self.ord {
                                SortOrder::NameAsc => SortOrder::NameDesc,
                                SortOrder::NameDesc => SortOrder::CmcAsc,
                                SortOrder::CmcAsc => SortOrder::CmcDesc,
                                SortOrder::CmcDesc => SortOrder::NameAsc,
                            }
                        }
                        SettingsSection::OpenIntoRecent => {
                            if let Some(f) = self.oir {
                                self.oir = Some(!f);
                            };
                        }
                        SettingsSection::Save => self.section = SettingsSection::Exit,
                        SettingsSection::Exit => self.section = SettingsSection::Save,
                    };
                    ViewExit::Hold
                }
                KeyCode::Left => {
                    match self.section {
                        SettingsSection::Tags => {
                            if self.vpos == 0 {
                                self.vpos = self.vt.len() - 1;
                            } else {
                                self.vpos -= 1;
                            }
                        }
                        SettingsSection::TagText => {
                            if self.tpos > 0 {
                                self.tpos -= 1;
                            }
                        }
                        SettingsSection::DefaultFilter => {
                            self.df = match self.df {
                                DefaultFilter::Name => DefaultFilter::Text,
                                DefaultFilter::Text => DefaultFilter::Name,
                            };
                        }
                        SettingsSection::Ordering => {
                            self.ord = match self.ord {
                                SortOrder::NameAsc => SortOrder::NameDesc,
                                SortOrder::NameDesc => SortOrder::CmcAsc,
                                SortOrder::CmcAsc => SortOrder::CmcDesc,
                                SortOrder::CmcDesc => SortOrder::NameAsc,
                            };
                        }
                        SettingsSection::OpenIntoRecent => {
                            if let Some(f) = self.oir {
                                self.oir = Some(!f);
                            };
                        }
                        SettingsSection::Save => self.section = SettingsSection::Exit,
                        SettingsSection::Exit => self.section = SettingsSection::Save,
                    };
                    ViewExit::Hold
                }
                KeyCode::Down => {
                    self.section = match self.section {
                        SettingsSection::Tags => SettingsSection::DefaultFilter,
                        SettingsSection::TagText => SettingsSection::TagText,
                        SettingsSection::DefaultFilter => SettingsSection::Ordering,
                        SettingsSection::Ordering => {
                            if self.oir == None {
                                SettingsSection::Save
                            } else {
                                SettingsSection::OpenIntoRecent
                            }
                        }
                        SettingsSection::OpenIntoRecent => SettingsSection::Save,
                        SettingsSection::Save => SettingsSection::Tags,
                        SettingsSection::Exit => SettingsSection::Tags,
                    };
                    ViewExit::Hold
                }
                KeyCode::Up => {
                    self.section = match self.section {
                        SettingsSection::Tags => SettingsSection::Save,
                        SettingsSection::TagText => SettingsSection::TagText,
                        SettingsSection::DefaultFilter => SettingsSection::Tags,
                        SettingsSection::Ordering => SettingsSection::DefaultFilter,
                        SettingsSection::OpenIntoRecent => SettingsSection::Ordering,
                        SettingsSection::Save => {
                            if self.oir == None {
                                SettingsSection::Ordering
                            } else {
                                SettingsSection::OpenIntoRecent
                            }
                        }
                        SettingsSection::Exit => {
                            if self.oir == None {
                                SettingsSection::Ordering
                            } else {
                                SettingsSection::OpenIntoRecent
                            }
                        }
                    };
                    ViewExit::Hold
                }
                KeyCode::Enter => match self.section {
                    SettingsSection::Tags => {
                        if self.vt[self.vpos] != String::from("main")
                            || self
                                .vt
                                .iter()
                                .filter(|&s| s == &String::from("main"))
                                .count()
                                > 1
                        {
                            self.section = SettingsSection::TagText;
                            if self.vpos == self.vt.len() - 1 {
                                self.vt[self.vpos] = String::new();
                                self.wt = String::new();
                            } else {
                                self.wt = self.vt[self.vpos].clone();
                            }
                        }
                        ViewExit::Hold
                    }
                    SettingsSection::TagText => {
                        self.tpos = 0;
                        self.section = SettingsSection::Tags;
                        if self.vt[self.vpos].len() == 0 {
                            self.vt.remove(self.vpos);
                            self.vch.push(TagChange::DeleteTag(self.wt.clone()));
                        }
                        if self.wt.is_empty() {
                            self.vch
                                .push(TagChange::InsertTag(self.vt[self.vpos].clone()));
                            self.vt.sort();
                            self.vt.push(String::from("{Add new tag}"));
                        } else {
                            self.vch.push(TagChange::ChangeTag(
                                self.wt.clone(),
                                self.vt[self.vpos].clone(),
                            ));
                            let s = self.vt.pop().unwrap();
                            self.vt.sort();
                            self.vt.push(s.clone());
                        }
                        ViewExit::Hold
                    }
                    SettingsSection::DefaultFilter => ViewExit::Hold,
                    SettingsSection::Ordering => ViewExit::Hold,
                    SettingsSection::OpenIntoRecent => ViewExit::Hold,
                    SettingsSection::Save => {
                        let changes = Changes {
                            df: self.df,
                            so: self.ord,
                            oir: self.oir,
                            vtch: self.vch.clone(),
                        };
                        ViewExit::Save(changes)
                    }
                    SettingsSection::Exit => ViewExit::Cancel,
                },
                KeyCode::Delete => {
                    match self.section {
                        SettingsSection::Tags => {
                            if self.vpos < self.vt.len() - 1
                                && (self.vt[self.vpos] != String::from("main")
                                    || self
                                        .vt
                                        .iter()
                                        .filter(|&s| s == &String::from("main"))
                                        .count()
                                        > 1)
                            {
                                self.vch
                                    .push(TagChange::DeleteTag(self.vt[self.vpos].clone()));
                                self.vt.remove(self.vpos);
                            }
                        }
                        SettingsSection::TagText => {
                            let s = &mut self.vt[self.vpos];
                            if self.tpos < s.len() {
                                s.remove(self.tpos);
                            }
                        }
                        _ => {}
                    }
                    ViewExit::Hold
                }
                KeyCode::Backspace => {
                    if self.tpos > 0 {
                        self.vt[self.vpos].remove(self.tpos - 1);
                        self.tpos -= 1;
                    }
                    ViewExit::Hold
                }
                KeyCode::Char(c) => {
                    if self.section == SettingsSection::TagText {
                        if self.vt[self.vpos] != String::from("main")
                            || self
                                .vt
                                .iter()
                                .filter(|&s| s == &String::from("main"))
                                .count()
                                > 1
                        {
                            self.vt[self.vpos].insert(self.tpos, c);
                            self.tpos += 1;
                        }
                    }
                    ViewExit::Hold
                }
                _ => ViewExit::Hold,
            }
        }

        pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
            let mut st = self.vt.get(self.vpos).unwrap().clone();
            let mut vsp = Vec::new();
            for i in 0..self.vt.len() {
                let mut s = self.vt[i].clone();
                s.push(' ');
                if i == self.vpos {
                    let span = Span::styled(
                        s,
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    );
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
                    Span::styled(
                        s3,
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    ),
                    Span::styled(
                        s2,
                        Style::default()
                            .add_modifier(Modifier::UNDERLINED)
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    ),
                    Span::styled(
                        s1,
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    ),
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
            let mut dfp = Paragraph::new(dfpt).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Default Filter"),
            );

            let ordt = match self.ord {
                SortOrder::NameAsc => "Cards ordered by name ascending.",
                SortOrder::CmcAsc => "Cards ordered by mana cost ascending.",
                SortOrder::NameDesc => "Cards ordered by name descending.",
                SortOrder::CmcDesc => "Cards ordered by mana cost descending.",
            };
            let mut ordp = Paragraph::new(ordt).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Default Ordering"),
            );

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
                    ts = ts.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Tags")
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
                SettingsSection::TagText => {
                    ts = ts.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Tags")
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
                SettingsSection::DefaultFilter => {
                    dfp = dfp.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Default Filter")
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
                SettingsSection::Ordering => {
                    ordp = ordp.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Default Ordering")
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
                SettingsSection::OpenIntoRecent => {
                    oirp = oirp.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Quickstart?")
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
                SettingsSection::Save => {
                    sp = sp.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
                SettingsSection::Exit => {
                    cp = cp.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Yellow)),
                    );
                }
            }

            let theight = 3 + (length as u16 / frame.size().width);

            let mut vrct = Vec::new();
            let constraints = [
                Constraint::Length(theight),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ];
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
                    x: r.x - 1,
                    y: r.y - 1,
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

    impl CreateDeckView {
        pub fn new() -> Self {
            Self {
                section: CreateDeckSection::Title,
                title: String::new(),
                com1: String::new(),
                com2: String::new(),
                vcn: Vec::new(),
                vpos: ListState::default(),
                tpos: 0,
            }
        }

        pub fn handle_input(&mut self, c: KeyCode, conn: &Connection) -> ViewExit {
            let active = match self.section {
                CreateDeckSection::Title => &mut self.title,
                CreateDeckSection::PrimaryCommander => &mut self.com1,
                CreateDeckSection::SecondaryCommander => &mut self.com2,
            };

            match c {
                KeyCode::Esc => return ViewExit::Cancel,
                KeyCode::End => self.tpos = active.len(),
                KeyCode::Home => self.tpos = 0,
                KeyCode::Char(c) => {
                    active.insert(self.tpos, c);
                    self.tpos += 1;
                    if self.section != CreateDeckSection::Title {
                        let a = active.clone();
                        self.uvcn(&a, conn);
                    }
                }
                KeyCode::Backspace => {
                    if self.tpos > 0 {
                        active.remove(self.tpos - 1);
                        self.tpos -= 1;
                    }
                    if self.section != CreateDeckSection::Title {
                        let a = active.clone();
                        self.uvcn(&a, conn);
                    }
                }
                KeyCode::Delete => {
                    if self.tpos < active.len() {
                        active.remove(self.tpos);
                    }
                    if self.section != CreateDeckSection::Title {
                        let a = active.clone();
                        self.uvcn(&a, conn);
                    }
                }
                KeyCode::BackTab => {
                    self.section = CreateDeckSection::Title;
                    self.tpos = 0;
                    self.com1 = String::new();
                    self.com2 = String::new();
                }
                KeyCode::Up => {
                    if self.section != CreateDeckSection::Title && self.vcn.len() > 0 {
                        let i = match self.vpos.selected() {
                            Some(i) => {
                                if i == 0 {
                                    self.vcn.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        self.vpos.select(Some(i));
                    }
                }
                KeyCode::Down => {
                    if self.section != CreateDeckSection::Title && self.vcn.len() > 0 {
                        let i = match self.vpos.selected() {
                            Some(i) => (i + 1) % self.vcn.len(),
                            None => 0,
                        };
                        self.vpos.select(Some(i));
                    }
                }
                KeyCode::Left => {
                    if self.tpos > 0 {
                        self.tpos -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.tpos < active.len() {
                        self.tpos += 1;
                    }
                }
                KeyCode::Enter => {
                    self.tpos = 0;
                    match self.section {
                        CreateDeckSection::Title => {
                            self.section = CreateDeckSection::PrimaryCommander
                        }
                        CreateDeckSection::PrimaryCommander => {
                            if let Some(i) = self.vpos.selected() {
                                let c = rcfn(conn, &self.vcn[i], None).unwrap();
                                match c.is_commander() {
                                    super::CommanderType::Default => {
                                        let did =
                                            ideck(conn, &self.title, &c.name, None, "Commander")
                                                .unwrap();
                                        return ViewExit::NewDeck(did);
                                    }
                                    super::CommanderType::Partner => {
                                        self.com1 = c.name;
                                        self.section = CreateDeckSection::SecondaryCommander;
                                        self.vcn = Vec::new();
                                        self.vpos.select(None);
                                    }
                                    super::CommanderType::PartnerWith(scn) => {
                                        self.com1 = c.name;
                                        self.com2 = scn.clone();
                                        self.section = CreateDeckSection::SecondaryCommander;
                                        self.vcn = vec![scn];
                                        self.vpos.select(Some(0));
                                    }
                                    super::CommanderType::Invalid => {}
                                }
                            }
                        }
                        CreateDeckSection::SecondaryCommander => match self.vpos.selected() {
                            Some(i) => {
                                let did = ideck(
                                    conn,
                                    &self.title,
                                    &self.com1,
                                    Some(self.vcn[i].clone()),
                                    "Commander",
                                )
                                .unwrap();
                                return ViewExit::NewDeck(did);
                            }
                            None => {
                                let did = ideck(conn, &self.title, &self.com1, None, "Commander")
                                    .unwrap();
                                return ViewExit::NewDeck(did);
                            }
                        },
                    }
                }
                _ => {}
            }
            ViewExit::Hold
        }

        pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
            let mut vrct = Vec::new();
            let cut = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)])
                .split(frame.size());
            vrct.push(cut[0]);

            let cut = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(26), Constraint::Min(18)])
                .split(cut[1]);
            vrct.push(cut[1]);

            vrct.append(
                &mut Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Length(3),
                            Constraint::Length(3),
                        ]
                        .as_ref(),
                    )
                    .split(cut[0]),
            );

            let vli: Vec<ListItem> = self
                .vcn
                .iter()
                .map(|s| ListItem::new(s.to_string()))
                .collect();
            let list = List::new(vli)
                .block(
                    Block::default()
                        .title("Select Commander")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::ITALIC)
                        .fg(Color::Yellow),
                );
            let text = self.rstyle();

            let (title, com1, com2) = match self.section {
                CreateDeckSection::Title => {
                    let text = self.rstyle();
                    let title = Paragraph::new(text).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Deck Name")
                            .style(Style::default().fg(Color::Yellow)),
                    );
                    let com1 = Paragraph::new(self.com1.clone())
                        .block(Block::default().borders(Borders::ALL).title("Commander"));
                    let com2 = Paragraph::new(self.com2.clone()).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Secondary Commander"),
                    );
                    (title, com1, com2)
                }
                CreateDeckSection::PrimaryCommander => {
                    let title = Paragraph::new(self.title.clone())
                        .block(Block::default().borders(Borders::ALL).title("Deck Name"));
                    let com1 = Paragraph::new(text).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Commander")
                            .style(Style::default().fg(Color::Yellow)),
                    );
                    let com2 = Paragraph::new(self.com2.clone()).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Secondary Commander"),
                    );
                    (title, com1, com2)
                }
                CreateDeckSection::SecondaryCommander => {
                    let title = Paragraph::new(self.title.clone())
                        .block(Block::default().borders(Borders::ALL).title("Deck Name"));
                    let com1 = Paragraph::new(self.com1.clone())
                        .block(Block::default().borders(Borders::ALL).title("Commander"));
                    let com2 = Paragraph::new(text).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Secondary Commander")
                            .style(Style::default().fg(Color::Yellow)),
                    );
                    (title, com1, com2)
                }
            };

            frame.render_widget(title, vrct[0]);
            frame.render_stateful_widget(list, vrct[1], &mut self.vpos.clone());
            frame.render_widget(com1, vrct[2]);
            if self.section == CreateDeckSection::SecondaryCommander || !self.com2.is_empty() {
                frame.render_widget(com2, vrct[3]);
            }
        }

        fn uvcn(&mut self, active: &String, conn: &Connection) {
            let rvcn = if self.section == CreateDeckSection::SecondaryCommander {
                rvcnfnp(conn, &active)
            } else {
                rvcnfn(conn, &active)
            };
            self.vcn = match rvcn {
                Ok(vs) => {
                    if vs.len() > 0 {
                        self.vpos.select(Some(0));
                    } else {
                        self.vpos.select(None);
                    }
                    vs
                }
                Err(_) => {
                    self.vpos.select(None);
                    Vec::new()
                }
            }
        }

        fn rstyle(&self) -> Spans {
            let mut st = match self.section {
                CreateDeckSection::Title => self.title.clone(),
                CreateDeckSection::PrimaryCommander => self.com1.clone(),
                CreateDeckSection::SecondaryCommander => self.com2.clone(),
            };
            st.push(' ');
            let (s1, s2) = st.split_at(self.tpos);
            let (s2, s3) = s2.split_at(1);
            let vs = vec![
                Span::styled(
                    String::from(s1),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::styled(
                    String::from(s2),
                    Style::default()
                        .add_modifier(Modifier::UNDERLINED)
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::styled(
                    String::from(s3),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
            ];
            Spans::from(vs)
        }
    }

    impl OpenDeckView {
        pub fn new() -> Self {
            Self {
                decks: Vec::new(),
                state: Default::default(),
                deleting: false,
            }
        }

        pub fn init(&mut self, conn: &Connection) {
            let decks = rvd(conn).unwrap();
            if decks.len() > 0 {
                self.state.select(Some(0));
            }
            self.decks = decks;
            self.deleting = false;
        }

        pub fn handle_input(&mut self, c: KeyCode) -> OpenDeckViewExit {
            match c {
                KeyCode::Up => self.previous(),
                KeyCode::Down => self.next(),
                KeyCode::Delete => {
                    if self.state.selected().is_some() {
                        self.deleting = true;
                        return OpenDeckViewExit::Hold
                    }
                },
                KeyCode::Enter => {
                    if self.deleting {
                        self.deleting = false;
                        if let Some(d) = self.remove() {
                            let did = d.id;
                            return OpenDeckViewExit::DeleteDeck(did);
                        }
                    } else {
                        if let Some(i) = self.state.selected() {
                            let did = self.decks.get(i).unwrap().id;
                            return OpenDeckViewExit::OpenDeck(did);
                        }
                    }
                },
                KeyCode::Esc => return OpenDeckViewExit::Cancel,
                _ => {}
            }
            self.deleting = false;
            OpenDeckViewExit::Hold
        }

        pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
            let table = self.rdt();
            frame.render_stateful_widget(table, frame.size(), &mut self.state.clone());

            if self.deleting {
                if let Some(i) = self.state.selected() {
                    let deck = &self.decks[i].name;
                    // let rect = Layout::default()
                    //     .direction(Direction::Horizontal)
                    //     .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    //     .split(frame.size());
                    let title = "Confirm Deletion";
                    let message = format!("Are you sure you want to delete the below deck?\n{deck}\nPress Enter to confirm.");
                    let err_message = Paragraph::new(message)
                        .block(Block::default().borders(Borders::ALL).title(title));
                    let area = centered_rect(60, frame.size());
                    frame.render_widget(tui::widgets::Clear, area);
                    frame.render_widget(err_message, area);
                };
            }
        }

        fn remove(&mut self) -> Option<Deck> {
            if self.decks.len() == 0 {
                return None;
            }
            let a = self.state.selected().unwrap();
            let d = self.decks.remove(a);
    
            if self.decks.len() == 0 {
                self.state = TableState::default();
            } else if self.decks.len() == a {
                self.state.select(Some(a - 1));
            }
    
            Some(d)
        }

        fn next(&mut self) {
            if self.decks.len() == 0 {
                return;
            }
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

        fn previous(&mut self) {
            if self.decks.len() == 0 {
                return;
            }
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

        fn rdt(&self) -> Table {
            let decks = self.decks.clone();
            let headers = Row::new(vec![
                Cell::from("ID"),
                Cell::from("Deck Name"),
                Cell::from("Commander(s)"),
                Cell::from("Color"),
            ])
            .style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            );
            let mut rows = Vec::new();

            for deck in decks {
                let (height, com2) = match deck.commander2 {
                    Some(c) => (2, c.name),
                    None => (1, String::new()),
                };

                let r = Row::new(vec![
                    Cell::from(deck.id.to_string()),
                    Cell::from(deck.name),
                    Cell::from(format!("{}\n{}", deck.commander.name, com2)),
                    Cell::from(deck.color),
                ])
                .height(height)
                .style(Style::default());

                rows.push(r);
            }

            let table = Table::new(rows)
                .header(headers)
                .block(Block::default().borders(Borders::ALL))
                .widths(&[
                    Constraint::Length(4),
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                    Constraint::Length(7),
                ])
                .column_spacing(1)
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                );

            table
        }
    }

    impl DeckView {
        pub fn new(
            did: i32,
            settings: Rc<RefCell<DeckSettings>>,
            dbc: Arc<Mutex<Connection>>,
        ) -> DeckView {
            let deck = rdfdid(&dbc.lock().unwrap(), did).unwrap();
            let cf = CardFilter::from(
                did,
                &deck.color,
                settings.borrow().df,
                settings.borrow().ordering,
            );
            let st = settings
                .borrow()
                .tags
                .iter()
                .position(|s| s == &String::from("main"))
                .unwrap();
            let vcdec = rvcnfcf(&dbc.lock().unwrap(), &cf.make_query(false, "")).unwrap();

            let mut slde = StatefulList::with_items(vcdec.clone());
            let name = slde.next().unwrap();
            let ac = Some(rcfn(&dbc.lock().unwrap(), &name, Some(cf.did)).unwrap());
            let sldb = StatefulList::default();

            DeckView {
                omni: String::new(),
                omniprev: String::new(),
                omnipos: 0,
                vsomni: Vec::new(),
                slde,
                sldb,
                vcdec,
                st,
                ac,
                cf,
                dvs: DeckViewSection::DeckOmni,
                settings,
                dbc,
            }
        }

        pub fn handle_input(&mut self, c: KeyCode) -> DeckViewExit {
            match self.dvs {
                DeckViewSection::DeckOmni | DeckViewSection::DbOmni => match c {
                    KeyCode::Left => {
                        if self.omnipos > 0 {
                            self.omnipos -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.omnipos < self.omni.len() {
                            self.omnipos += 1;
                        }
                    }
                    KeyCode::Up => {
                        if let Some(i) = self.vsomni.iter().position(|s| s == &self.omni) {
                            if i > 0 {
                                self.omni = self.vsomni[i - 1].clone();
                                self.omnipos = self.omni.len();
                                if self.dvs == DeckViewSection::DeckOmni {
                                    self.uvc();
                                }
                            }
                        } else {
                            if let Some(s) = self.vsomni.last() {
                                self.omni = s.clone();
                                self.omnipos = self.omni.len();
                                if self.dvs == DeckViewSection::DeckOmni {
                                    self.uvc();
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(i) = self.vsomni.iter().position(|s| s == &self.omni) {
                            if i < self.vsomni.len() - 1 {
                                self.omni = self.vsomni[i + 1].clone();
                                self.omnipos = self.omni.len();
                                if self.dvs == DeckViewSection::DeckOmni {
                                    self.uvc();
                                }
                            } else {
                                self.omni = String::new();
                                self.omnipos = 0;
                                if self.dvs == DeckViewSection::DeckOmni {
                                    self.uvc();
                                }
                            }
                        }
                    }
                    KeyCode::Home => self.omnipos = 0,
                    KeyCode::End => self.omnipos = self.omni.len(),
                    KeyCode::Delete => {
                        if self.omnipos < self.omni.len() {
                            self.omni.remove(self.omnipos);
                        }
                        if self.dvs == DeckViewSection::DeckOmni {
                            self.uvc();
                        }
                    }
                    KeyCode::Backspace => {
                        if self.omnipos > 0 {
                            self.omni.remove(self.omnipos - 1);
                            self.omnipos -= 1;
                        }
                        if self.dvs == DeckViewSection::DeckOmni {
                            self.uvc();
                        }
                    }
                    KeyCode::Enter => {
                        let so = self.omni.trim();
                        if so == "/stat" {
                            return DeckViewExit::Stats;
                        } else if so == "/settings" || so == "/config" {
                            return DeckViewExit::Settings(self.cf.did);
                        } else {
                            let mut tag = String::new();
                            let re = regex::Regex::new(r"/tag:(\w*)").unwrap();
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
                            self.omnipos = self.omnipos.min(self.omni.len());

                            if omni.len() > 0 {
                                if let Some(i) = self.vsomni.iter().position(|s| s == &omni) {
                                    self.vsomni.remove(i);
                                };
                                self.vsomni.push(omni.clone());
                            }

                            if self.dvs == DeckViewSection::DbOmni {
                                self.uvc();
                                if self.sldb.state.selected() != None {
                                    self.dvs = DeckViewSection::DbCards;
                                }
                            } else if self.slde.items.len() > 0 {
                                self.dvs = DeckViewSection::DeckCards;
                            }

                            if tag.len() > 0 {
                                return DeckViewExit::NewTag(tag, self.cf.did);
                            }
                        }
                    }
                    KeyCode::Esc => return DeckViewExit::MainMenu,
                    KeyCode::Tab => {
                        (self.omni, self.omniprev) = (self.omniprev.clone(), self.omni.clone());
                        self.omnipos = 0;
                        if self.dvs == DeckViewSection::DbOmni {
                            self.dvs = DeckViewSection::DeckOmni;
                            self.uac();
                        } else {
                            self.dvs = DeckViewSection::DbOmni;
                            self.uac();
                        }
                    }
                    KeyCode::Char(c) => {
                        self.omni.insert(self.omnipos, c);
                        self.omnipos += 1;
                        if self.dvs == DeckViewSection::DeckOmni {
                            self.uvc();
                        }
                    }
                    _ => {}
                },
                DeckViewSection::DeckCards | DeckViewSection::DbCards => {
                    let sl = match self.dvs {
                        DeckViewSection::DeckCards => &mut self.slde,
                        DeckViewSection::DbCards => &mut self.sldb,
                        _ => todo!(),
                    };

                    match c {
                        KeyCode::Up => {
                            sl.previous();
                            self.uac();
                        }
                        KeyCode::Down => {
                            sl.next();
                            self.uac();
                        }
                        KeyCode::Right => {
                            self.st += 1;
                            if self.st >= self.settings.borrow().tags.len() {
                                self.st = 0;
                            }
                        }
                        KeyCode::Left => {
                            if self.st > 0 {
                                self.st -= 1;
                            } else {
                                self.st = self.settings.borrow().tags.len() - 1;
                            }
                        }
                        KeyCode::Delete => {
                            let cn = sl.get().unwrap();
                            let c = rcfn(&self.dbc.lock().unwrap(), cn, None).unwrap();
                            let d = rdfdid(&self.dbc.lock().unwrap(), self.cf.did).unwrap();
                            if d.commander == c {
                                return DeckViewExit::Hold;
                            }
                            if let Some(com) = d.commander2 {
                                if com == c {
                                    return DeckViewExit::Hold;
                                }
                            };

                            if let Some(j) = self.vcdec.iter().position(|s| s == cn) {
                                self.vcdec.remove(j);
                                let flag = self.dvs == DeckViewSection::DeckCards;
                                dcntodc(&self.dbc.lock().unwrap(), cn, self.cf.did).unwrap();

                                if flag {
                                    sl.remove();
                                }

                                match &c.lo {
                                    CardLayout::Paired(_, _, n) => {
                                        if flag {
                                            sl.remove_named(n);
                                        }
                                        dcntodc(&self.dbc.lock().unwrap(), &n, self.cf.did)
                                            .unwrap();
                                    }
                                    CardLayout::Meld(s, n, m) => {
                                        if flag {
                                            sl.remove_named(m);
                                        }
                                        let _a =
                                            dcntodc(&self.dbc.lock().unwrap(), &m, self.cf.did);

                                        if s == &'b' {
                                            if flag {
                                                sl.remove_named(n);
                                            }
                                            dcntodc(&self.dbc.lock().unwrap(), &n, self.cf.did)
                                                .unwrap();
                                        }
                                    }
                                    _ => {}
                                }

                                self.vcdec = rvcnfcf(
                                    &self.dbc.lock().unwrap(),
                                    &self.cf.make_query(false, ""),
                                )
                                .unwrap();

                                if sl.state.selected() == None {
                                    //Can only happen in Deck view
                                    self.ac = None;
                                    self.dvs = DeckViewSection::DeckOmni;
                                }

                                if !flag {
                                    //The deck is dirty, vectors need to be refreshed.
                                    let vc = rvcnfcf(
                                        &self.dbc.lock().unwrap(),
                                        &self.cf.make_query(false, &self.omniprev),
                                    )
                                    .unwrap();
                                    self.slde.reinitialize(vc, true);
                                } else {
                                    self.uac();
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cn = &self.ac.as_ref().unwrap().name;
                            if self.vcdec.contains(cn) {
                                self.toggle_tag()
                            } else {
                                if let Ok(vc) = ictodc(
                                    &self.dbc.lock().unwrap(),
                                    &self.ac.as_ref().unwrap(),
                                    self.cf.did,
                                ) {
                                    for c in vc {
                                        self.vcdec.push(c.name);
                                    }
                                }
                                let vc = rvcnfcf(
                                    &self.dbc.lock().unwrap(),
                                    &self.cf.make_query(false, &self.omniprev),
                                )
                                .unwrap();
                                self.slde.reinitialize(vc, true);
                            }
                        }
                        KeyCode::Esc => return DeckViewExit::MainMenu,
                        KeyCode::Tab => {
                            if self.dvs == DeckViewSection::DeckCards {
                                self.dvs = DeckViewSection::DeckOmni;
                            } else {
                                self.dvs = DeckViewSection::DbOmni;
                            }
                        }
                        KeyCode::Char(' ') => self.uacr(),
                        KeyCode::Char('u') => {
                            if let Some(ac) = &self.ac {
                                if ac.stale {
                                    if let Ok(card) = upfcn_detailed(
                                        &self.dbc.lock().unwrap(),
                                        &ac,
                                        Some(self.cf.did),
                                    ) {
                                        self.ac = Some(card);
                                    }
                                }
                            };
                        }
                        _ => {}
                    }
                }
            }
            DeckViewExit::Hold
        }

        pub fn uct(&mut self, changes: Vec<TagChange>) {
            self.cf.df = self.settings.borrow().df;
            self.cf.so = self.settings.borrow().ordering;
            if self.st >= self.settings.borrow().tags.len() {
                self.st = self
                    .settings
                    .borrow()
                    .find_tag(&String::from("main"))
                    .unwrap();
            }
            for tc in changes {
                utindc(&self.dbc.lock().unwrap(), tc, &self.cf)
            }
        }

        pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
            let tag_max = self
                .settings
                .borrow()
                .tags
                .iter()
                .map(|s| s.len())
                .max()
                .unwrap()
                + 2;
            let mut vrct = Vec::new();
            let cut = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
                .split(frame.size());

            vrct.append(
                &mut Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Min(20),
                            Constraint::Max(tag_max.try_into().unwrap()),
                        ]
                        .as_ref(),
                    )
                    .split(cut[0]),
            );

            vrct.append(
                &mut Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(26), Constraint::Min(3)].as_ref())
                    .split(cut[1]),
            );

            let bdef = Block::default().borders(Borders::ALL);
            let bfoc = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let mut _bomni = Block::default();
            let mut _blist = Block::default();

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

            let (vli, ls) = match self.dvs {
                DeckViewSection::DeckOmni => {
                    // _bomni = bfoc.title(format!("Tag max: {}", tag_max));
                    _bomni = bfoc.title("Filter Deck");
                    _blist = bdef
                        .clone()
                        .title(format!("Deck View ({})", self.slde.items.len()));
                    let vli = self.slde.rvli();
                    (vli, &self.slde.state)
                }
                DeckViewSection::DeckCards => {
                    _bomni = bdef.clone().title("Filter Deck");
                    _blist = bfoc.title(format!("Deck View ({})", self.slde.items.len()));
                    let vli = self.slde.rvli();
                    (vli, &self.slde.state)
                }
                DeckViewSection::DbOmni => {
                    _bomni = bfoc.title("Filter Database");
                    _blist = bdef
                        .clone()
                        .title(format!("Database View ({})", self.sldb.items.len()));
                    let vli = self.sldb.rvlis(&self.vcdec);
                    (vli, &self.sldb.state)
                }
                DeckViewSection::DbCards => {
                    _bomni = bdef.clone().title("Filter Database");
                    _blist = bfoc.title(format!("Database View ({})", self.sldb.items.len()));
                    let vli = self.sldb.rvlis(&self.vcdec);
                    (vli, &self.sldb.state)
                }
            };

            let tag = &self.settings.borrow().tags[self.st];
            let po = Paragraph::new(Spans::from(spans)).block(_bomni);
            let pt = Paragraph::new(tag.clone()).block(bdef.clone());
            let pc = match &self.ac {
                Some(card) => card.display().block(bdef.clone()),
                None => Paragraph::new("No card found!").block(bdef.clone()),
            };
            let lc = List::new(vli)
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                )
                .block(_blist);

            frame.render_widget(po, vrct[0]);
            frame.render_widget(pt, vrct[1]);
            frame.render_stateful_widget(lc, vrct[2], &mut ls.clone());
            frame.render_widget(pc, vrct[3]);
        }

        fn insert_tag(&mut self, tag: String) {
            self.settings.borrow_mut().add_tag(tag.clone());
            self.st = self.settings.borrow().find_tag(&tag).unwrap();
        }

        fn toggle_tag(&mut self) {
            let cn = self.ac.as_ref().unwrap().to_string();
            self.ac = ttindc(
                &self.dbc.lock().unwrap(),
                &cn,
                &self.settings.borrow().tags[self.st],
                self.cf.did,
            );
        }

        fn uvc(&mut self) {
            let (sl, general) = match self.dvs {
                DeckViewSection::DeckOmni | DeckViewSection::DeckCards => (&mut self.slde, false),
                DeckViewSection::DbOmni | DeckViewSection::DbCards => (&mut self.sldb, true),
            };

            let vc = rvcnfcf(
                &self.dbc.lock().unwrap(),
                &self.cf.make_query(general, &self.omni),
            )
            .unwrap();
            sl.reinitialize(vc, false);
            self.uac();
        }

        fn uac(&mut self) {
            let mm = String::new(); //this is dumb, but it works. Otherwise complains of temp value dropped.
            let cn = match self.dvs {
                DeckViewSection::DeckOmni | DeckViewSection::DeckCards => {
                    self.slde.get().unwrap_or(&mm)
                }
                DeckViewSection::DbOmni | DeckViewSection::DbCards => {
                    self.sldb.get().unwrap_or(&mm)
                }
            };
            if cn == &mm {
                self.ac = None;
            } else {
                self.ac = Some(rcfn(&self.dbc.lock().unwrap(), cn, Some(self.cf.did)).unwrap());
            }
        }

        fn uacr(&mut self) {
            let c = self.ac.as_ref().unwrap();
            let cn = match &c.lo {
                CardLayout::Paired(_, _, n) => n,
                CardLayout::Meld(s, n, m) => {
                    if s == &'b' {
                        n
                    } else {
                        let meld = rcfn(&self.dbc.lock().unwrap(), &m, None).unwrap();
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
                }
                _ => {
                    return;
                }
            };

            self.ac =
                Some(crate::db::rcfn(&self.dbc.lock().unwrap(), cn, Some(self.cf.did)).unwrap());
        }
    }
}
