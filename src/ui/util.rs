use std::collections::HashMap;

use crossterm::event::KeyCode;
use regex::Regex;
use tui::{layout::Constraint, text::{Span, Spans}, widgets::{BarChart, Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap}};
use tui::style::{Color, Modifier, Style};

use crate::{CardStat, db};

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

pub struct StatefulList<T: ToString> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T: ToString> StatefulList<T> {

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

    //TODO: Never going to apply, but check for len here as well?
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

    pub fn rvlis(& self,  pl: Vec<crate::Card>) -> Vec<ListItem> {
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
    pub lc: List<'a>,
    pub fc: Paragraph<'a>,
    len: usize,
}

impl<'a> DeckScreen<'a> {
    pub fn new(omnitext: Spans<'a>, vli: Vec<ListItem<'a>>, cardtext: String, mode: Screen) -> DeckScreen<'a> {
        let (omni_title, list_title) = match mode {
            Screen::DeckOmni | Screen::DeckCard => { ("Filter Deck", "Card List") }
            Screen::DbFilter | Screen::DbCards => { ("Filter Database", "Database") }
            _ => { panic!(); }
        };
        
        let len = vli.len();
        let input = Paragraph::new(omnitext)
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title(omni_title));
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