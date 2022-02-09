use std::env;
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::db::*;
use crate::util::views::*;
use crate::util::*;
use anyhow::Result;
use crossterm::{
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rusqlite::Connection;
use tui::backend::CrosstermBackend;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List};
use tui::Terminal;

struct AppState {
    mode: Screen,
    mode_p: Screen,
    deck_view: Option<DeckView>,
    settings_view: Option<SettingsView>,
    deck_stat_view: Option<DeckStatView>,
    create_deck_view: CreateDeckView,
    open_deck_view: OpenDeckView,
    slmm: StatefulList<MainMenuItem>,
    dbc: Arc<Mutex<Connection>>,
    settings: Settings,
    quit: bool,
}

impl AppState {
    fn new() -> AppState {
        let p = get_local_file("settings.toml", true);
        let file_settings = FileSettings::new(&p).unwrap();
        let settings = Settings::from(file_settings);
        let p = get_local_file("lieutenant.db", true);
        let conn = Connection::open(p).unwrap();

        add_regexp_function(&conn).unwrap();
        let mut app = AppState {
            mode: Screen::MainMenu,
            mode_p: Screen::MainMenu,
            deck_view: None,
            settings_view: None,
            deck_stat_view: None,
            create_deck_view: CreateDeckView::new(),
            open_deck_view: OpenDeckView::new(),
            slmm: StatefulList::new(),
            dbc: Arc::new(Mutex::new(conn)),
            settings,
            quit: false,
        };

        app.init_main_menu();

        if let Some(did) = app.settings.get_oir() {
            app.init_deck_view(did);
        }

        app
    }

    fn handle_input(&mut self, c: KeyCode) -> Result<()> {
        match self.mode {
            Screen::MainMenu => match c {
                KeyCode::Esc => {
                    self.quit = true;
                }
                KeyCode::Up => {
                    self.slmm.previous();
                }
                KeyCode::Down => {
                    self.slmm.next();
                }
                KeyCode::Enter => {
                    self.switch_mode(self.slmm.get().unwrap().next);
                }
                _ => {}
            },
            Screen::OpenDeck => {
                let res = self.open_deck_view.handle_input(c);
                match res {
                    OpenDeckViewExit::Hold => {}
                    OpenDeckViewExit::Cancel => self.mode = Screen::MainMenu,
                    OpenDeckViewExit::OpenDeck(did) => {
                        self.init_deck_view(did);
                    }
                    OpenDeckViewExit::DeleteDeck(did) => {
                        dd(&self.dbc.lock().unwrap(), did).unwrap();
                        self.settings.dd(did);
                    }
                }
            }
            Screen::MakeDeck => {
                let res = self
                    .create_deck_view
                    .handle_input(c, &self.dbc.lock().unwrap());
                match res {
                    ViewExit::NewDeck(did) => {
                        self.settings.id(did);
                        self.create_deck_view = CreateDeckView::new();
                        self.init_deck_view(did);
                    }
                    ViewExit::Cancel => {
                        self.mode = {
                            self.create_deck_view = CreateDeckView::new();
                            Screen::MainMenu
                        }
                    }
                    _ => {}
                }
            }
            Screen::DeckStat => {
                self.mode = Screen::DeckView;
            }
            Screen::Settings => {
                if let Some(sv) = &mut self.settings_view {
                    match sv.handle_input(c) {
                        views::ViewExit::Save(changes) => {
                            if self.mode_p == Screen::MainMenu {
                                self.settings.change(&changes, None);
                                self.mode = Screen::MainMenu
                            } else {
                                self.settings.change(
                                    &changes,
                                    Some(self.deck_view.as_ref().unwrap().rdid()),
                                );
                                self.deck_view.as_mut().unwrap().uct(changes.vtch);
                                self.mode_p = Screen::Settings;
                                self.mode = Screen::DeckView;
                            }
                        }
                        views::ViewExit::Cancel => {
                            if self.mode_p == Screen::DeckView {
                                self.mode_p = Screen::Settings;
                                self.mode = Screen::DeckView;
                            } else {
                                self.mode = Screen::MainMenu
                            }
                        }
                        _ => {} //impossible
                    }
                }
            }
            Screen::DeckView => {
                let a = self.deck_view.as_mut().unwrap().handle_input(c);
                match a {
                    DeckViewExit::Hold => {}
                    DeckViewExit::MainMenu => self.mode = Screen::MainMenu,
                    DeckViewExit::Stats => {
                        self.deck_stat_view = Some(DeckStatView::new(
                            self.dbc.clone(),
                            self.deck_view.as_ref().unwrap().rdid(),
                        ));
                        self.mode = Screen::DeckStat
                    }
                    DeckViewExit::Settings(did) => {
                        self.mode_p = Screen::DeckView;
                        self.init_settings(Some(did));
                    }
                    DeckViewExit::NewTag(s, did) => self.settings.it(Some(did), s.to_string()),
                }
            }
        }

        Ok(())
    }

    fn switch_mode(&mut self, next: Option<Screen>) {
        match next {
            Some(Screen::MakeDeck) => {
                self.mode = Screen::MakeDeck;
            }
            Some(Screen::OpenDeck) => {
                self.init_open_view();
            }
            Some(Screen::Settings) => {
                self.init_settings(None);
            }
            Some(Screen::MainMenu) => {
                self.mode = Screen::MainMenu;
            }
            Some(Screen::DeckView) => {
                let did = self.settings.rr();
                self.init_deck_view(did);
            }
            Some(_) => {}
            None => self.quit = true,
        }
    }

    fn init_create_view(&mut self) {}

    fn init_deck_view(&mut self, did: i32) {
        self.settings.sr(did);

        self.deck_view = Some(DeckView::new(did, self.settings.rds(did), self.dbc.clone()));
        self.mode = Screen::DeckView;
    }

    fn init_settings(&mut self, odid: Option<i32>) {
        let sv = match odid {
            Some(did) => {
                let d = rdfdid(&self.dbc.lock().unwrap(), did).unwrap();
                views::SettingsView::new(
                    self.settings.get_tags_deck(did),
                    self.settings.rdf(Some(did)),
                    self.settings.rso(Some(did)),
                    format!("Deck Settings for {}", d.name),
                    None,
                )
            }
            None => views::SettingsView::new(
                self.settings.get_tags(),
                self.settings.rdf(None),
                self.settings.rso(None),
                String::from("Global Settings"),
                Some(false),
            ),
        };
        self.settings_view = Some(sv);
        self.mode = Screen::Settings;
    }

    fn init_open_view(&mut self) {
        self.mode = Screen::OpenDeck;
        self.open_deck_view.init(&self.dbc.lock().unwrap());
        // self.stod.init(&self.dbc.lock().unwrap());
    }

    fn init_main_menu(&mut self) {
        let mut items = Vec::new();
        if self.settings.rr() > 0 {
            items.push(MainMenuItem::from_with_screen(
                String::from("Load most recent deck"),
                Screen::DeckView,
            ));
        }
        items.push(MainMenuItem::from_with_screen(
            String::from("Create a new deck"),
            Screen::MakeDeck,
        ));
        items.push(MainMenuItem::from_with_screen(
            String::from("Load a deck"),
            Screen::OpenDeck,
        ));
        items.push(MainMenuItem::from_with_screen(
            String::from("Settings"),
            Screen::Settings,
        ));
        items.push(MainMenuItem::from(String::from("Quit")));

        self.slmm = StatefulList::with_items(items);
        self.slmm.next();
    }

    // pub fn render(&mut self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _a = terminal.draw(|frame| {
            match self.mode {
                Screen::MainMenu => {
                    let list = List::new(self.slmm.rvli())
                        .block(Block::default().title("Main Menu").borders(Borders::ALL))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Cyan),
                        );
    
                    frame.render_stateful_widget(list, frame.size(), &mut self.slmm.state.clone());
                },
                Screen::DeckView => self.deck_view.as_ref().unwrap().render(frame),
                Screen::Settings => self.settings_view.as_ref().unwrap().render(frame),
                Screen::MakeDeck => self.create_deck_view.render(frame),
                Screen::OpenDeck => self.open_deck_view.render(frame),
                Screen::DeckStat => {
                    if let Some(dsv) = &mut self.deck_stat_view {
                        dsv.recalc();
                        dsv.render(frame);
                    }
                }
            }
        });
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let mut state = AppState::new();
    // let mut widgets = WidgetOwner::new();

    loop {
        state.render(&mut terminal);

        if state.mode != Screen::DeckStat {
            if let Event::Key(KeyEvent { code, .. }) = read()? {
                let _a = state.handle_input(code);
                if state.mode == Screen::DeckStat {
                    let did = state.deck_view.as_ref().unwrap().rdid();
                    let arc = Arc::clone(&state.dbc);
                    thread::spawn(move || {
                        ucfd(&arc, did).unwrap();
                    });
                }
            }
        } else {
            if poll(Duration::from_millis(100))? {
                if let Event::Key(KeyEvent { code, .. }) = read()? {
                    let _a = state.handle_input(code);
                }
            }
        }

        if state.quit {
            break;
        }
    }

    let mut p = env::current_exe().unwrap();
    p.pop();
    p.push("settings.toml");
    std::fs::write(p, state.settings.to_toml()).unwrap();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
