

use std::io::{self, Write};
use tui::Terminal;
use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};
use tui::backend::CrosstermBackend;
use crossterm::{cursor, terminal, ClearType, Result, input, AlternateScreen, InputEvent, KeyEvent, RawScreen};
use crossterm::IntoRawMode;
use structopt::StructOpt;
use std::sync::mpsc;
use std::thread;

pub fn draw() -> Result<()> {
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

    loop {
        app(&mut terminal)?;
        let mut should_quit = false;
        match rx.recv().unwrap() {
            InputEvent::Keyboard(k) => {
                match k {
                    KeyEvent::Esc => {
                        should_quit = true;
                        println!("Program Exiting");
                    },
                    _ => {}
                }
            },
            _ => {}
        }

        if should_quit { break; }
    }

    Ok(())
}

fn app(terminal: &mut Terminal<CrosstermBackend>) -> Result<()> {

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
             .title("Block 1")
             .borders(Borders::ALL)
             .render(&mut f, area1[0]);
        Block::default()
             .title("Block 2")
             .borders(Borders::ALL)
             .render(&mut f, area1[1]);
        Block::default()
             .title("Block 3")
             .borders(Borders::ALL)
             .render(&mut f, area2[0]);
        Block::default()
             .title("Block 4")
             .borders(Borders::ALL)
             .render(&mut f, area2[1]);
    })?;

    Ok(())

}