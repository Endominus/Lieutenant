use Card;
use std::io;
use tui::Terminal;
use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};
use tui::backend::CrosstermBackend;
use tui::style::{Color, Modifier, Style};
use crossterm::{input, InputEvent, KeyEvent, RawScreen};

use std::sync::mpsc;
use std::thread;
// use std::time::Duration;

// use crate::db;

#[derive(Clone, PartialEq)]
enum Content {
    SearchString(String),
    Results(Vec<Card>),
    Selected(Card),
    Tags(Vec<String>),
    None
}

#[derive(Clone, PartialEq)]
struct State {
    title: String,
    content: Content,
    focus: bool,
}

impl State {
    fn new(title: String, ) -> State {
        State { title, content: Content::None, focus: false}
    }

    fn focus(&mut self) {
        self.focus = true;
    }

    fn unfocus(&mut self) {
        self.focus = false;
    }

    fn handle_input(&mut self, k: KeyEvent) {
        match &self.content {
            Content::SearchString(s) => {
                match k {
                    KeyEvent::Char(c) => match c {
                        '\t' => {},
                        _ => {},
                    },
                    _ => {}
                }
            },
            Content::Results(vc) => {
                match k {
                    KeyEvent::Char(c) => match c {
                        '\t' => {},
                        _ => {},
                    },
                    _ => {}
                }
            },
            Content::Selected(c) => {
                match k {
                    KeyEvent::Char(c) => match c {
                        '\t' => {},
                        _ => {},
                    },
                    _ => {}
                }
            },
            Content::Tags(vs) => {
                match k {
                    KeyEvent::Char(c) => match c {
                        '\t' => {},
                        _ => {},
                    },
                    _ => {}
                }
            },
            Content::None => {},
        }

    }
}

struct App {
    search_block: Vec<State>,
    search_position: usize,
    result_block: Vec<State>,
    card_block: State,
    other_block: Vec<State>,
    deck_id: usize,
    quit: bool,
}

impl App {
    fn new(deck_id: usize, sv: & mut Vec<State>) -> App {
            let s3 = State::new(String::from("Advanced Search"));
            let s2 = State::new(String::from("Search by text"));
            let mut s1 = State::new(String::from("Search by name"));
            s1.focus();
            sv.push(s1);
            sv.push(s2);
            sv.push(s3);


            let r1 = State::new(String::from("Results"));
            let rv = vec![r1];

            let c1 = State::new(String::from("Card Info"));
            
            let o1 = State::new(String::from("Tags"));
            let ov = vec![o1];
        App {
            search_block: sv.to_vec(),
            search_position: 0,
            result_block: rv,
            card_block: c1,
            other_block: ov,
            deck_id,
            quit: false,
        }
    }

    fn focus_next(&mut self) {
        if self.search_block[self.search_position].focus {
            self.search_block[self.search_position].unfocus();
            self.result_block[0].focus();
        } else if self.result_block[0].focus {
            self.result_block[0].unfocus();
            self.other_block[0].focus();
        } else {
            self.other_block[0].focus();
            self.search_block[self.search_position].focus();
        }
    }

    fn handle_input(&mut self, k: KeyEvent) {
        match k {
            KeyEvent::Esc => {
                self.quit = true;
            },
            KeyEvent::CtrlDown => {
                self.focus_next();
            },
            KeyEvent::Char(c) => match c {
                '\t' =>  {
                        self.search_block[self.search_position].unfocus();
                        self.search_position = (self.search_position + 1) % 3;
                        self.search_block[self.search_position].focus();
                    }
                _ => {}
            }
            _ => { self.handle_input(k); }
        }
    }
}

fn draw(terminal: &mut Terminal<CrosstermBackend>, app: &App) -> Result<(), io::Error> {

    terminal.draw(|mut f| {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(60),
                    Constraint::Percentage(40)
                ].as_ref()
            )
            .split(f.size());
        let area1 = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Max(3),
                    Constraint::Percentage(50)
                ].as_ref()
            )
            .split(chunks[0]);
        let area2 = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(50)
                ].as_ref()
            )
            .split(chunks[1]);
        


        let mut style1 = Style::default();
        let mut style2 = Style::default();
        let mut style3 = Style::default();

        if app.search_block[app.search_position].focus {
            style1 = Style::default().fg(Color::Yellow);
        } else if app.result_block[0].focus {
            style2 = Style::default().fg(Color::Yellow);
        } else {
            style3 = Style::default().fg(Color::Yellow);
        }

        Block::default()
             .title(&app.search_block[app.search_position].title)
             .borders(Borders::ALL)
             .border_style(style1)
             .render(&mut f, area1[0]);
        Block::default()
             .title(&app.result_block[0].title)
             .borders(Borders::ALL)
             .border_style(style2)
             .render(&mut f, area1[1]);
        Block::default()
             .title(&app.card_block.title)
             .borders(Borders::ALL)
             .render(&mut f, area2[0]);
        Block::default()
             .title(&app.other_block[0].title)
             .borders(Borders::ALL)
             .border_style(style3)
             .render(&mut f, area2[1]);
    })?;

    Ok(())

}

pub fn run(deck_id: usize) -> Result<(), failure::Error> {
        let _screen = RawScreen::into_raw_mode();
    let (tx, rx) = mpsc::channel();
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let input = input();
            let mut reader = input.read_sync();
            loop {
                let event = reader.next();

                if let Some(key_event) = event {
                    let _a = tx.send(key_event);
                }
            }
        });
    }

    let backend = CrosstermBackend::new();
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;


    let mut sv = vec![];
    let mut app = App::new(deck_id, &mut sv);

    loop {
        draw(&mut terminal, &app)?;
        match rx.recv()? {
            InputEvent::Keyboard(k) => {
                app.handle_input(k);
            },
            _ => {}
        }
        if app.quit { break; }
    }

    Ok(())
}

// fn key_handler(app:&mut App, rx:& std::sync::mpsc::Receiver<crossterm::InputEvent>) -> bool {
//     let mut should_quit = false;
//     match rx.recv().unwrap() {
//         InputEvent::Keyboard(k) => {
//             match k {
//                 KeyEvent::Esc => {
//                     should_quit = true;
//                 },
//                 KeyEvent::CtrlDown => {
//                     app.focus_next();
//                 },
//                 KeyEvent::Char(c) => match c {
//                     '\t' =>  {
//                             app.search_position = (app.search_position + 1) % 3;
//                         }
//                     _ => {}
//                 }
//                 _ => { app.handle_input(k); }
//             }
//         },
//         _ => {}
//     }
//     should_quit
// }