use Card;
use std::io;
use tui::Terminal;
use tui::widgets::{Widget, Block, Borders, SelectableList, Paragraph, Text};
use tui::layout::{Layout, Constraint, Direction};
use tui::backend::CrosstermBackend;
use tui::style::{Color, Modifier, Style};
use crossterm::{input, InputEvent, KeyEvent, RawScreen, TerminalCursor};

use std::sync::mpsc;
use std::thread;
// use std::time::Duration;

// use crate::db;

#[derive(Clone, PartialEq)]
enum Content<'a> {
    SearchString(String),
    Results(Vec<Card>, usize),
    Selected(&'a Card),
    Tags(Vec<String>),
    None
}

#[derive(Clone, PartialEq)]
struct State<'a> {
    title: String,
    content: Content<'a>,
    focus: bool,
}

impl<'a> State<'a> {
    fn new(title: String, ) -> State<'a> {
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
                let mut s = s.clone();
                match k {
                    KeyEvent::Char(c) => { 
                        s.insert(s.len(), c);
                    },
                    KeyEvent::Backspace => { s.pop(); },
                    //KeyEvent::Enter => { //TODO: Implement adding the current string to the list}
                    //},
                    _ => {}
                }
                self.content = Content::SearchString(s);
            },
            Content::Results(vc, mut index) => {
                match k {
                    KeyEvent::Down => {
                        if index < vc.len()-1 { index += 1; }
                    },
                    KeyEvent::Up => {
                        if index > 0 { index -= 1; }
                    },
                    _ => {}
                }
                self.content = Content::Results(vc.to_vec(), index);
            },
            Content::Selected(c) => {},
            Content::Tags(vs) => {},
            Content::None => {},
        }
    }

    fn selected(&self) -> Option<&Card> {
        match &self.content {
            Content::Results(vc, s) => { vc.get(*s) }
            _ => None
        }
    }
}

struct App<'a> {
    search_block: Vec<State<'static>>,
    search_position: usize,
    result_block: Vec<State<'static>>,
    card_block: &'a mut State<'a>,
    other_block: Vec<State<'static>>,
    deck_id: i32,
    quit: bool,
}

impl<'a> App<'a> {
    fn new(
        deck_id: i32, 
        sv: &mut Vec<State<'static>>,
        rv: &'a mut Vec<State<'static>>,
        cs: &'a mut State<'a>,
        ov: &mut Vec<State<'static>>,
        ) -> App<'a> {
            use db;

            let mut s1 = State::new(String::from("Search by name"));
            let mut s2 = State::new(String::from("Search by text"));
            let mut s3 = State::new(String::from("Advanced Search"));

            s1.content = Content::SearchString("".to_string());
            s2.content = Content::SearchString("".to_string());
            s3.content = Content::SearchString("".to_string());

            s1.focus();

            sv.push(s1);
            sv.push(s2);
            sv.push(s3);


            let mut r1 = State::new(String::from("Results"));
            let results = db::rvcd(deck_id).unwrap();
            r1.content = Content::Results(results, 0);
            // let rv = vec![r1];
            rv.push(r1);

            // let mut c1 = State::new(String::from("Card Info"));
            cs.content = Content::Selected(rv[0].selected().unwrap());
            
            let o1 = State::new(String::from("Tags"));
            // let ov = vec![o1];
            ov.push(o1);
        App {
            search_block: sv.to_vec(),
            search_position: 0,
            result_block: rv.to_vec(),
            card_block: &mut *cs,
            other_block: ov.to_vec(),
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
            _ => {
                if self.search_block[self.search_position].focus {
                    self.search_block[self.search_position].handle_input(k);
                } else {
                    self.result_block[0].handle_input(k);
                }
            }
            // KeyEvent::Char(c) => match c {
            //     '\t' =>  {
            //             self.search_block[self.search_position].unfocus();
            //             self.search_position = (self.search_position + 1) % 3;
            //             self.search_block[self.search_position].focus();
            //         }
            //     _ => {}
            // }
            // _ => { self.handle_input(k); }
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

        if let Content::Results(vc, s) = &app.result_block[0].content {
            let mut vn: Vec<String> = vec![];

            for c in vc {
                vn.push(c.name.clone());
            }

            SelectableList::default()
                .block(Block::default().title(&app.result_block[0].title).border_style(style2).borders(Borders::ALL))
                .items(&vn)
                .select(Some(*s))
                .highlight_style(Style::default().fg(Color::Yellow).modifier(Modifier::BOLD))
                .highlight_symbol(">")
                .render(&mut f, area1[1]);
        // }
            // .items();
        // if let Content::Selected(c) = &app.card_block.content {
                let info = vc[*s].ri();
                let mut text = Vec::new();

                for l in info {
                    text.push(Text::raw(l));
                    text.push(Text::raw("\n"));
                }

                Paragraph::new(text.iter())
                    .block(Block::default()
                        .title(&app.card_block.title)
                        .borders(Borders::ALL)
                        .border_style(Style::default()))
                    .wrap(true)
                    .render(&mut f, area2[0]);
                    // .items(&info)
        }
        Block::default()
             .title(&app.other_block[0].title)
             .borders(Borders::ALL)
             .border_style(style3)
             .render(&mut f, area2[1]);
    })?;

    Ok(())

}

pub fn run(deck_id: i32) -> Result<(), failure::Error> {
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
    let cursor = TerminalCursor::new();
    cursor.hide()?;
    terminal.clear()?;


    let mut sv = vec![];
    let mut rv = vec![];
    let mut ov = vec![];
    let mut cs = State::new(String::from("Card Info"));
    let mut app = App::new(deck_id, &mut sv, &mut rv, &mut cs, &mut ov,);

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