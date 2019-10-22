use Card;
use std::io;
use tui::Terminal;
use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};
use tui::backend::CrosstermBackend;
use crossterm::{input, InputEvent, KeyEvent, RawScreen};

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::db;

#[derive(Clone)]
enum Content {
    SearchString(String),
    Results(Vec<Card>),
    Selected(Card),
    Tags(Vec<String>),
    None
}

#[derive(Clone)]
struct State {
    title: String,
    content: Content
}

impl State {
    fn new(title: String, ) -> State {
        State { title, content: Content::None}
    }
}

struct App<'a> {
    search_block: Vec<State>,
    search_position: usize,
    result_block: Vec<State>,
    card_block: State,
    other_block: Vec<State>,
    focus: &'a State,
    deck_id: usize,
}

impl<'a> App<'a> {
    fn new(deck_id: usize, sv: &'a mut Vec<State>) -> App<'a> {
            let s3 = State::new(String::from("Advanced Search"));
            let s2 = State::new(String::from("Search by text"));
            let s1 = State::new(String::from("Search by name"));
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
            focus: &sv[0],
            deck_id
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
                    Constraint::Min(1),
                    Constraint::Percentage(90)
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
        Block::default()
             .title(&app.search_block[app.search_position].title)
             .borders(Borders::ALL)
             .render(&mut f, area1[0]);
        Block::default()
             .title(&app.result_block[0].title)
             .borders(Borders::ALL)
             .render(&mut f, area1[1]);
        Block::default()
             .title(&app.card_block.title)
             .borders(Borders::ALL)
             .render(&mut f, area2[0]);
        Block::default()
             .title(&app.other_block[0].title)
             .borders(Borders::ALL)
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
        let mut should_quit = false;
        match rx.recv()? {
            InputEvent::Keyboard(k) => {
                match k {
                    KeyEvent::Esc => {
                        should_quit = true;
                        // println!("Program Exiting");
                    },
                    KeyEvent::Char(c) => match c {
                        '\t' =>  {
                                app.search_position = (app.search_position + 1) % 3;
                            }
                        _ => {}
                    }
                    _ => {}
                }
            },
            _ => {}
        }

        if should_quit { break; }
    }

    Ok(())
}