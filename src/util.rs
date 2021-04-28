// use std::collections::HashMap;

// use crossterm::event::KeyCode;

use regex::Regex;
use rusqlite::Connection;
use tui::{layout::Constraint, text::{Span, Spans}, widgets::{BarChart, Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap}};
use tui::style::{Color, Modifier, Style};
// use lazy_static::lazy_static;

use std::{collections::HashMap, path::PathBuf};
use serde::Deserialize;
use serde_derive::Serialize;
use config::{Config, ConfigError};
use itertools::Itertools;

#[derive(Debug)]
pub enum SortOrder {
    NameAsc,
    NameDesc,
    CmcAsc,
    CmcDesc,
}

pub enum DefaultFilter {
    Name,
    Text
}

#[derive(Debug, Deserialize, Serialize)]
struct SettingsGroup {
    tags: Option<Vec<String>>,
    ordering: Option<String>,
    default_filter: Option<String>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    global: SettingsGroup,
    decks: HashMap<i32, SettingsGroup>
}

impl Settings {
    pub fn new(path: &PathBuf) -> Result<Self, ConfigError> {
        let mut s = Config::default();
        s.merge(config::File::with_name(path.to_str().unwrap())).unwrap();

        s.try_into()
    }

    pub fn get_tags(&self) -> Vec<String> {
        self.global.tags.as_ref().unwrap().clone()
    }

    pub fn get_tags_deck(&self, deck: i32) -> Vec<String> {
        let mut r = Vec::new();
        r.append(&mut self.global.tags.as_ref().unwrap().clone());
        if let Some(s) = self.decks.get(&deck) {
            if let Some(t) = &s.tags {
                r.append(&mut t.clone());
            };
        };
        r
    }

    pub fn get_sort_order(&self, deck: i32) -> SortOrder {
        if let Some(d) = self.decks.get(&deck) {
            if let Some(o) = &d.ordering {
                match o.as_str() {
                    "+name" => { return SortOrder::NameAsc }
                    "-name" => { return SortOrder::NameDesc }
                    "+cmc" => { return SortOrder::CmcAsc }
                    "-cmc" => { return SortOrder::CmcDesc }
                    _ => { return SortOrder::NameAsc }
                }
            }
        }

        if let Some(o) = &self.global.ordering {
            match o.as_str() {
                "+name" => { return SortOrder::NameAsc }
                "-name" => { return SortOrder::NameDesc }
                "+cmc" => { return SortOrder::CmcAsc }
                "-cmc" => { return SortOrder::CmcDesc }
                _ => { return SortOrder::NameAsc }
            }
        }

        SortOrder::NameAsc
    }

    pub fn get_default_filter(&self, deck: i32) -> DefaultFilter {
        if let Some(d) = self.decks.get(&deck) {
            if let Some(f) = &d.default_filter {
                match f.as_str() {
                    "text" => { return DefaultFilter::Text }
                    _ => { return DefaultFilter::Name }
                }
            }
        }

        if self.global.default_filter == Some(String::from("text")) { return DefaultFilter::Text }
        else { return DefaultFilter::Name }
    }

    pub fn add_tag(&mut self, deck: i32, tag: String) -> Option<Vec<String>> {
        let mut vt = self.get_tags_deck(deck);
        let mut o = None;
        let mut df = None;
        if !vt.contains(&tag) {
            let mut nvt = Vec::from([tag]);
            if let Some(d) = &mut self.decks.get(&deck) {
                o = d.ordering.clone();
                df = d.ordering.clone();
                if let Some(vt) = &d.tags {
                    nvt.append(&mut vt.clone());
                    nvt.sort();
                }
            }
            let d = SettingsGroup { tags: Some(nvt.clone()), ordering: o, default_filter: df };
            self.decks.insert(deck, d);
            vt.append(&mut nvt);
            return Some(vt)
        }

        None
    }

    pub fn to_toml(&self) -> String {
        let mut vr = Vec::from([String::from("[global]")]);
        if let Some(vt) = &self.global.tags {
            vr.push(String::from("tags = ["));
            for t in vt {
                vr.push(format!("\t\"{}\",", t));
            }
            vr.push(String::from("]"));
        }
        if let Some(o) = &self.global.ordering {
            vr.push(format!("ordering = \"{}\"", o));
        }
        if let Some(df) = &self.global.default_filter {
            vr.push(format!("default_filter = \"{}\"", df));
        }
        vr.push(String::from("\n[decks]"));

        // Explore using a BTreeMap instead to lose dependence on itertools
        let vk = self.decks.keys().sorted();
        
        for k in vk {
            let v = self.decks.get(k).unwrap();
            vr.push(format!("\t[decks.{}]", k));
            if let Some(vt) = &v.tags {
                vr.push(String::from("\ttags = ["));
                for t in vt {
                    vr.push(format!("\t\t\"{}\",", t));
                }
                vr.push(String::from("\t]"));
            }
            if let Some(o) = &v.ordering {
                vr.push(format!("\tordering = \"{}\"", o));
            }
            if let Some(df) = &v.default_filter {
                vr.push(format!("\tdefault_filter = \"{}\"", df));
            }
            vr.push(String::new());
        }
    
        vr.join("\n")
    }
}

// lazy_static! {
//     static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::new().unwrap());
// }


#[derive(Copy, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    MakeDeck,
    OpenDeck,
    Settings,
    DeckOmni,
    DeckCard,
    DbFilter,
    DbCards,
    DeckStat,
    Error(&'static str),
}

// pub enum WidgetState {
//     Lis
// }

pub struct StatefulList<T: ToString + PartialEq> {
    // pub state: ListState,
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
            // let fs = f.to_string();
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
            Cell::from("Deck Name"),
            Cell::from("Commander(s)"),
            Cell::from("Color"),
        ])
        .style(Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Cyan));
        let mut rows = Vec::new();
        // rows.push(headers);

        for deck in decks {
            let (height, com2) = match deck.commander2 {
                Some(c) => { (2, c.name) }
                None => { (1, String::new()) }
            };

            let r = Row::new(vec![
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
            .block(Block::default().title("Open Deck")
                .borders(Borders::ALL))
            .widths(&[Constraint::Percentage(40), Constraint::Percentage(40), Constraint::Length(7)])
            .column_spacing(1)
            .highlight_style(Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC));

        table
    }

    pub fn next(&mut self) {
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
        if let Some(i) = self.state.selected() {
            self.decks.get(i)
        } else {
            None
        }
    }

    pub fn remove(&mut self) -> Option<Deck> {
        let a = self.state.selected().unwrap();
        if self.decks.len() > 0 {
            if a+1 == self.decks.len() { self.state.select(Some(a-1)); }
            // else { self.state.select(Some(a+1)); }
        } else {
            self.state = TableState::default();
        }
        Some(self.decks.remove(a))
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

#[derive(Clone)]
pub enum MakeDeckFocus {
    Title,
    Commander,
    SecondaryCommander,
    // Type
}

impl Default for MakeDeckFocus {fn default() -> Self { Self::Title } }

#[derive(Default)]
pub struct MakeDeckContents {
    pub focus: MakeDeckFocus,
    pub title: String,
    pub commander: String,
    pub commander2: String,
    pub commander_names: Vec<String>,
}

// impl MakeDeckContents {}
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

pub struct DeckScreen<'a> {
    pub omni: Paragraph<'a>,
    pub tags: Paragraph<'a>,
    pub lc: List<'a>,
    pub fc: Paragraph<'a>,
    len: usize,
}

impl<'a> DeckScreen<'a> {
    pub fn new(
        omnitext: Spans<'a>, 
        tag: &StatefulList<String>,
        vli: Vec<ListItem<'a>>, 
        cardtext: String, 
        mode: Screen) -> DeckScreen<'a> {
        let (omni_title, list_title) = match mode {
            Screen::DeckOmni | Screen::DeckCard => { ("Filter Deck", "Card List") }
            Screen::DbFilter | Screen::DbCards => { ("Filter Database", "Database") }
            _ => { panic!(); }
        };
        
        let len = vli.len();
        let input = Paragraph::new(omnitext)
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title(omni_title));
        let tag = Paragraph::new(Span::from(tag.get().unwrap().clone()))
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Tags"));
        let list = List::new(vli)
            .block(Block::default().title(list_title).borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
        let card = Paragraph::new(cardtext)
            .style(Style::default())
            .wrap(Wrap { trim: false } )
            .block(Block::default().borders(Borders::ALL).title("Card Info"));
            
        DeckScreen {
            omni: input,
            tags: tag,
            lc: list,
            fc: card,
            len
        }
    }

    pub fn focus_omni(&mut self, mode: Screen) {
        let (omni_title, list_title) = match mode {
            Screen::DeckOmni | Screen::DeckCard => { ("Filter Deck", format!("Card List ({})", self.len)) }
            Screen::DbFilter | Screen::DbCards => { ("Filter Database", format!("Database ({})", self.len)) }
            _ => { panic!(); }
        };

        let nb = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(omni_title);
        self.omni = self.omni.clone().block(nb);
        let nb = Block::default().title(list_title).borders(Borders::ALL);
        self.lc = self.lc.clone().block(nb);
    }

    pub fn focus_lc(&mut self, mode: Screen) {
        let (omni_title, list_title) = match mode {
            Screen::DeckOmni | Screen::DeckCard => { ("Filter Deck", format!("Card List ({})", self.len)) }
            Screen::DbFilter | Screen::DbCards => { ("Filter Database", format!("Database ({})", self.len)) }
            _ => { panic!(); }
        };

        let nb = Block::default()
            .borders(Borders::ALL)
            .title(omni_title);
        self.omni = self.omni.clone().block(nb);
        let nb = Block::default()
            .title(list_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        self.lc = self.lc.clone().block(nb);
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
            .bar_style(Style::default().fg(Color::White).bg(Color::Black))
            .value_style(Style::default().fg(Color::Black).add_modifier(Modifier::BOLD))
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

// pub fn rdt(conn: &Connection) -> Table {
    
// }

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
    pub lo: Layout,
    //TODO: Add rarity and sets
}

impl ToString for Card { fn to_string(& self) -> String { self.name.clone() } }
impl Card {
    pub fn ri(&self) -> Vec<String> {
        let mut v = vec![
            self.name.clone(),
            format!("{}, ({})", self.mana_cost, self.cmc),
            self.types.clone(),
            String::new(),
        ];
            
        let t = self.text.split("\n");
        for l in t {
            v.push(l.to_string());
        }

        if self.power.len() > 0 {
            v.push(format!("Power/Toughness: {}/{}", self.power, self.toughness));
        } else if self.loyalty.len() > 0 {
            v.push(format!("Loyalty: {}", self.loyalty.clone()));
        }

        // v.push(String::new());

        match &self.lo {
            Layout::Adventure(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Also has Adventure: {}", rel)); } 
                    'b' => { v.push(format!("Adventure of: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::Aftermath(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Also has Aftermath: {}", rel)); } 
                    'b' => { v.push(format!("Aftermath of: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::Flip(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Also has Flip side: {}", rel)); } 
                    'b' => { v.push(format!("Flip side of: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::ModalDfc(_, rel) => { v.push(format!("You may instead cast: {}", rel)); }
            Layout::Split(_, rel) => { v.push(format!("You may instead cast: {}", rel)); }
            Layout::Transform(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Transforms into: {}", rel)); } 
                    'b' => { v.push(format!("Transforms from: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::Meld(side, face, meld) => { 
                match side { 
                    'a' => { v.push(format!("Melds with {} to form {}", face, meld)); } 
                    'b' => { v.push(format!("Melds from {} and {}", face, meld)); } 
                    _ => {} 
                }
            }
            _ => {}
        }
        
        v.push(String::new());


        if self.tags.len() > 0 {
            v.push(format!("Tags: {}", self.tags.join(" ")));
        }
        v
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    Adventure(char, String),
    Aftermath(char, String),
    Flip(char, String),
    Leveler,
    Meld(char, String, String),
    ModalDfc(char, String),
    Normal,
    Saga,
    Split(char, String),
    Transform(char, String),
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Normal
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Relation {
    Single(String),
    Meld {face: String, transform: String },
}

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