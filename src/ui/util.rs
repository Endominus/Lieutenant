use crossterm::event::KeyCode;
use tui::widgets::{List, ListItem, ListState, Paragraph, Block, Borders, Wrap};
use tui::style::{Color, Modifier, Style};

use crate::db;

#[derive(Copy, Clone)]
pub enum Screen {
    MainMenu,
    DbFilter,
    DbCards,
    DeckOmni,
    DeckCard,
    OpenDeck,
    Settings,
    MakeDeck,
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

    pub fn next(&mut self) {
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
    }

    pub fn previous(&mut self) {
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

    pub fn rvli(& self) -> Vec<ListItem> {
        self.items.iter().map(|f| ListItem::new(f.to_string())).collect()
    }

    pub fn rvlis(& self,  pl: &Vec<crate::NewCard>) -> Vec<ListItem> {
        let vs: Vec<String> = pl.iter().map(|f| f.to_string()).collect();

        self.items.iter().map(|f| 
            // let fs = f.to_string();
            if vs.contains(&f.to_string()) {
                ListItem::new(f.to_string()).style(Style::default().fg(Color::Yellow))
            } else {
                ListItem::new(f.to_string())
            }
        ).collect()
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
    // Type
}

impl Default for MakeDeckFocus {fn default() -> Self { Self::Title } }

#[derive(Default)]
pub struct MakeDeckContents {
    pub focus: MakeDeckFocus,
    pub title: String,
    pub commander: String,
}

// impl MakeDeckContents {}
pub struct MakeDeckScreen<'a> {
    pub title_entry: Paragraph<'a>,
    pub commander_entry: Paragraph<'a>,
}

impl<'a> MakeDeckScreen<'a> {
    pub fn new(mdc: &MakeDeckContents) -> MakeDeckScreen<'a> {
        let (te, ce) = match mdc.focus {
            MakeDeckFocus::Title => {
                (Paragraph::new(mdc.title.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Deck Name")
                        .style(Style::default().fg(Color::Yellow))),
                Paragraph::new(mdc.commander.clone())
                .style(Style::default())
                .block(Block::default().borders(Borders::ALL).title("Commander")))
            }
            MakeDeckFocus::Commander => {
                (Paragraph::new(mdc.title.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Deck Name")
                        .style(Style::default().fg(Color::Cyan))),
                Paragraph::new(mdc.commander.clone())
                    .style(Style::default())
                    .block(Block::default().borders(Borders::ALL).title("Commander"))
                        .style(Style::default().fg(Color::Yellow)))
            }
        };

        MakeDeckScreen {
            title_entry: te,
            commander_entry: ce,
        }
    }
}

pub struct DeckScreen<'a> {
    pub omni: Paragraph<'a>,
    pub lc: List<'a>,
    pub fc: Paragraph<'a>,
    len: usize,
}

impl DeckScreen<'_> {
    pub fn new(omnitext: String, vli: Vec<ListItem>, cardtext: String, mode: Screen) -> DeckScreen {
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

// fn focus_border