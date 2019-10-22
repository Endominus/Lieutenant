use std::io;
use tui::Terminal;
use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};
use tui::backend::CrosstermBackend;
use crossterm::{input, InputEvent, KeyEvent, RawScreen};

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
struct State {
    title: String,
}

impl State {
    fn new(title: String, ) -> State {
        State { title, }
    }
}

struct App<'a> {
    search_block: &'a State,
    search_position: usize,
    result_block: State,
    card_block: State,
    other_block: State,
}

impl App<'_> {
    fn new(
        search_block: &State, 
        result_block: State, 
        card_block: State, 
        other_block: State) -> App {
        App {
            search_block,
            search_position: 0,
            result_block,
            card_block,
            other_block
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
             .title(&app.search_block.title)
             .borders(Borders::ALL)
             .render(&mut f, area1[0]);
        Block::default()
             .title(&app.result_block.title)
             .borders(Borders::ALL)
             .render(&mut f, area1[1]);
        Block::default()
             .title(&app.card_block.title)
             .borders(Borders::ALL)
             .render(&mut f, area2[0]);
        Block::default()
             .title(&app.other_block.title)
             .borders(Borders::ALL)
             .render(&mut f, area2[1]);
    })?;

    Ok(())

}

pub fn run() -> Result<(), failure::Error> {
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
                    tx.send(key_event);
                }
            }
        });
    }

    let backend = CrosstermBackend::new();
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    let s3 = State::new(String::from("Advanced Search"));
    let s2 = State::new(String::from("Search by text"));
    let s1 = State::new(String::from("Search by name"));

    let sv = vec![s1, s2, s3];
    // s3.next = Some(Box::new(s1));
    let r1 = State::new(String::from("Results"));
    let c1 = State::new(String::from("Card Info"));
    let o1 = State::new(String::from("Tags"));

    let mut app = App::new(& sv[0], r1, c1, o1);

    loop {
        draw(&mut terminal, &app)?;
        let mut should_quit = false;
        match rx.recv()? {
            InputEvent::Keyboard(k) => {
                match k {
                    KeyEvent::Esc => {
                        should_quit = true;
                        println!("Program Exiting");
                    },
                    KeyEvent::Char(c) => match c {
                        '\t' =>  {
                                app.search_position = (app.search_position + 1) % sv.len();
                                app.search_block = & sv[app.search_position.clone()];
                                // .unwrap_or(Box::new(State::clone(s1)));
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