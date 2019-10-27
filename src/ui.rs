use crossterm::{input, InputEvent, KeyEvent, RawScreen, TerminalCursor};
use std::io;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use tui::Terminal;

use Card;

use std::sync::mpsc;
use std::thread;
// use std::time::Duration;

// use crate::db;

#[derive(Clone, PartialEq)]
enum Content<'a> {
    SearchString(String, Vec<String>, usize),
    Results(Vec<Card>, usize),
    Selected(&'a Card),
    Tags(Vec<String>),
    None,
}

#[derive(Clone, PartialEq)]
struct State<'a> {
    title: String,
    content: Content<'a>,
    focus: bool,
}

impl<'a> State<'a> {
    fn new(title: String) -> State<'a> {
        State {
            title,
            content: Content::None,
            focus: false,
        }
    }

    fn focus(&mut self) {
        self.focus = true;
    }

    fn unfocus(&mut self) {
        self.focus = false;
    }

    fn handle_input(&mut self, k: KeyEvent) {
        match &self.content {
            Content::SearchString(s, vs, i) => {
                let mut s = s.clone();
                match k {
                    KeyEvent::Char(c) => {
                        s.insert(s.len(), c);
                    }
                    KeyEvent::Backspace => {
                        s.pop();
                    }
                    //KeyEvent::Enter => { //TODO: Implement adding the current string to the list}
                    //},
                    _ => {}
                }
                self.content = Content::SearchString(s, vs.to_vec(), *i);
            }
            Content::Results(vc, mut index) => {
                match k {
                    KeyEvent::Down => {
                        if index < vc.len() - 1 {
                            index += 1;
                        }
                    }
                    KeyEvent::Up => {
                        if index > 0 {
                            index -= 1;
                        }
                    }
                    _ => {}
                }
                self.content = Content::Results(vc.to_vec(), index);
            }
            Content::Selected(c) => {}
            Content::Tags(vs) => {}
            Content::None => {}
        }
    }

    fn selected(&self) -> Option<&Card> {
        match &self.content {
            Content::Results(vc, s) => vc.get(*s),
            _ => None,
        }
    }
}

struct App<'a> {
    sb: Vec<State<'static>>,
    sp: usize,
    rb: Vec<State<'static>>,
    rp: usize,
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

        s1.content = Content::SearchString("".to_string(), Vec::new(), 0);
        s2.content = Content::SearchString("".to_string(), Vec::new(), 0);
        s3.content = Content::SearchString("".to_string(), Vec::new(), 0);

        s1.focus();

        sv.push(s1);
        sv.push(s2);
        sv.push(s3);

        let mut r1 = State::new(String::from("Results"));
        let mut r2 = State::new(String::from("All Cards"));
        let results = db::rvcd(deck_id).unwrap();
        // let all_cards = db
        r1.content = Content::Results(results, 0);
        r2.content = Content::Results(Vec::new(), 0);
        // let rv = vec![r1];
        rv.push(r1);
        rv.push(r2);

        // let mut c1 = State::new(String::from("Card Info"));
        cs.content = Content::Selected(rv[0].selected().unwrap());

        let o1 = State::new(String::from("Tags"));
        // let ov = vec![o1];
        ov.push(o1);
        App {
            sb: sv.to_vec(),
            sp: 0,
            rb: rv.to_vec(),
            rp: 0,
            card_block: &mut *cs,
            other_block: ov.to_vec(),
            deck_id,
            quit: false,
        }
    }

    fn focus_next(&mut self) {
        if self.sb[self.sp].focus {
            self.sb[self.sp].unfocus();
            self.rb[self.rp].focus();
        } else if self.rb[self.rp].focus {
            self.rb[self.rp].unfocus();
            self.other_block[0].focus();
        } else {
            self.other_block[0].focus();
            self.sb[self.sp].focus();
        }
    }

    fn handle_input(&mut self, k: KeyEvent) {
        match k {
            KeyEvent::Esc => {
                self.quit = true;
            }
            KeyEvent::CtrlDown => {
                self.focus_next();
            }
            KeyEvent::Char(c) => match c {
                '\t' => {
                    if self.sb[self.sp].focus {
                        self.sb[self.sp].unfocus();
                        self.sp = (self.sp + 1) % 3;
                        self.sb[self.sp].focus();
                    } else {
                        self.rb[self.rp].unfocus();
                        self.rp = (self.rp + 1) % 2;
                        self.rb[self.rp].focus();
                    }
                }
                _ => {
                    if self.sb[self.sp].focus {
                        self.sb[self.sp].handle_input(k);
                        if let Content::SearchString(s, vs, i) = &self.sb[self.sp].content {
                            let results = rvcq(
                                s, 
                                &self.sb[self.sp].title, 
                                &self.rb[self.rp].title, 
                                self.deck_id);
                            if let Some(vc) = results {
                                self.rb[self.rp].content = Content::Results(vc, 0);
                            }
                        }
                    } else {
                        self.rb[self.rp].handle_input(k);
                    }
                }
            },
            _ => {
                if self.sb[self.sp].focus {
                    self.sb[self.sp].handle_input(k);
                    if let Content::SearchString(s, _vs, _i) = &self.sb[self.sp].content {
                        let results = rvcq(
                            s, 
                            &self.sb[self.sp].title, 
                            &self.rb[self.rp].title, 
                            self.deck_id);
                        if let Some(vc) = results {
                            self.rb[self.rp].content = Content::Results(vc, 0);
                        }
                    }
                } else {
                    self.rb[self.rp].handle_input(k);
                }
            }
        }
    }

    fn irb(&mut self) {
        
    }
}

fn rvcq(s: &String, st: &str, rt: &str, did: i32,) -> Option<Vec<Card>> {
    use db;
    let mut did = did;
    if rt == "All Cards" { did = -1; }

    match st {
        "Advanced Search" => { None },
        "Search by text" => Some(db::rvct(s.to_string(), did).unwrap()),
        "Search by name" => Some(db::rvcn(s.to_string(), did).unwrap()),
        _ => { None }
    }
    // unimplemented!()
}

fn draw(terminal: &mut Terminal<CrosstermBackend>, app: &App) -> Result<(), io::Error> {
    terminal.draw(|mut f| {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
            .split(f.size());
        let area1 = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(3), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);
        let area2 = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[1]);

        let mut style1 = Style::default();
        let mut style2 = Style::default();
        let mut style3 = Style::default();

        if app.sb[app.sp].focus {
            style1 = Style::default().fg(Color::Yellow);
        } else if app.rb[app.rp].focus {
            style2 = Style::default().fg(Color::Yellow);
        } else {
            style3 = Style::default().fg(Color::Yellow);
        }

        if let Content::SearchString(s, vs, i) = 
            &app.sb[app.sp].content {
            
            let mut text = "".to_string();
            if *i > 0 {
                text = vs[*i].clone();
            } else {
                text = s.to_string();
            }
            let mut v = vec![];

            v.push(Text::raw(text));

            Paragraph::new(v.iter())
                .block(
                    Block::default()
                        .title(&app.sb[app.sp].title)
                        .borders(Borders::ALL)
                        .border_style(style1))
                .scroll(0)
                .render(&mut f, area1[0]);
        }

        if let Content::Results(vc, s) = &app.rb[app.rp].content {
            let mut vn: Vec<String> = vec![];

            for c in vc {
                vn.push(c.name.clone());
            }

            SelectableList::default()
                .block(
                    Block::default()
                        .title(&app.rb[app.rp].title)
                        .border_style(style2)
                        .borders(Borders::ALL),
                )
                .items(&vn)
                .select(Some(*s))
                .highlight_style(Style::default().fg(Color::Yellow).modifier(Modifier::BOLD))
                .highlight_symbol(">")
                .render(&mut f, area1[1]);
            let mut info = Vec::new();
            if vc.len() > 0 {
                info = vc[*s].ri();
            }
            let mut text = Vec::new();

            for l in info {
                text.push(Text::raw(l));
                text.push(Text::raw("\n"));
            }

            Paragraph::new(text.iter())
                .block(
                    Block::default()
                        .title(&app.card_block.title)
                        .borders(Borders::ALL)
                        .border_style(Style::default()),
                )
                .wrap(true)
                .render(&mut f, area2[0]);
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
    let input = input();

    let backend = CrosstermBackend::new();
    let mut terminal = Terminal::new(backend)?;
    let cursor = TerminalCursor::new();
    cursor.hide()?;
    terminal.clear()?;

    let mut sv = vec![];
    let mut rv = vec![];
    let mut ov = vec![];
    let mut cs = State::new(String::from("Card Info"));
    let mut app = App::new(deck_id, &mut sv, &mut rv, &mut cs, &mut ov);

    loop {
        terminal.hide_cursor()?;
        draw(&mut terminal, &app)?;
        if let Some(e) = input.read_sync().next() {
            match e {
                InputEvent::Keyboard(k) => {
                    app.handle_input(k);
                }
                _ => {}
            }
        }
        if app.quit {
            break;
        }
    }

    Ok(())
}
