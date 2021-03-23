use tui::widgets::{List, ListItem, ListState, Paragraph, Block, Borders, Wrap};
use tui::style::{Color, Modifier, Style};
use anyhow::{Result, Error};

// use crate::Card;

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

pub struct DeckScreen<'a> {
    pub omni: Paragraph<'a>,
    pub lc: List<'a>,
    pub fc: Paragraph<'a>,
}

impl DeckScreen<'_> {
    pub fn new(omnitext: String, vli: Vec<ListItem>, cardtext: String) -> DeckScreen {
        let input = Paragraph::new(omnitext)
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Omnibar"));
        let list = List::new(vli)
            .block(Block::default().title("Card List").borders(Borders::ALL))
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
        }
    }

    pub fn focus_omni(&mut self) {
        let nb = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title("Omnibar");
        self.omni = self.omni.clone().block(nb);
        let nb = Block::default().title("Card List").borders(Borders::ALL);
        self.lc = self.lc.clone().block(nb);
    }

    pub fn focus_lc(&mut self) {
        let nb = Block::default()
            .borders(Borders::ALL)
            .title("Omnibar");
        self.omni = self.omni.clone().block(nb);
        let nb = Block::default()
            .title("Card List")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        self.lc = self.lc.clone().block(nb);
    }
}

// fn focus_border