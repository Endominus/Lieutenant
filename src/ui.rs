use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::db;
use crate::util::views::*;
use crate::util::*;
use anyhow::Result;
use crossterm::{
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use rusqlite::Connection;
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{BarChart, ListItem, Paragraph, Table, Wrap};
use tui::widgets::{Block, Borders, List};
use tui::Terminal;

struct AppState {
    mode: Screen,
    mode_p: Screen,
    deck_view: Option<DeckView>,
    settings_view: Option<SettingsView>,
    create_deck_view: Option<CreateDeckView>,
    did: i32,
    slmm: StatefulList<MainMenuItem>,
    stod: OpenDeckTable,
    dsi: DeckStatInfo,
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

        db::add_regexp_function(&conn).unwrap();
        let mut app = AppState {
            mode: Screen::MainMenu,
            mode_p: Screen::MainMenu,
            deck_view: None,
            settings_view: None,
            create_deck_view: None,
            did: -1,
            slmm: StatefulList::new(),
            stod: OpenDeckTable::default(),
            dsi: DeckStatInfo::default(),
            quit: false,
            dbc: Arc::new(Mutex::new(conn)),
            settings,
        };

        app.init_main_menu();
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
                match c {
                    KeyCode::Esc => {
                        self.mode = Screen::MainMenu;
                    }
                    KeyCode::Up => {
                        self.stod.previous();
                    }
                    KeyCode::Down => {
                        self.stod.next();
                    }
                    KeyCode::Enter => {
                        if let Some(deck) = self.stod.get() {
                            self.settings.sr(deck.id);
                            self.init_deck_view();
                        };
                    }
                    KeyCode::Delete => {
                        self.mode_p = self.mode;
                        self.mode = Screen::Error("Confirm Deletion\nAre you sure you want to delete the below deck?\n{DECK}\nPress Enter to confirm.");
                    }
                    _ => {}
                }
            }
            Screen::MakeDeck => {
                if let Some(cdv) = &mut self.create_deck_view {
                    let mut flag = false;
                    match cdv.handle_input(c, &self.dbc.lock().unwrap()) {
                        ViewExit::Save(_) => {}
                        ViewExit::NewDeck(did) => {
                            self.settings.sr(did);
                            self.settings.id(did);
                            flag = true;
                        }
                        ViewExit::Hold => {}
                        ViewExit::Cancel => self.mode = Screen::MainMenu,
                    }
                    if flag {
                        self.init_deck_view();
                    }
                }
            }
            Screen::DeckStat => {
                self.mode = Screen::DeckView;
            }
            Screen::Error(s) => {
                match c {
                    KeyCode::Enter => {
                        if s.starts_with("Confirm Deletion") {
                            if let Some(deck) = self.stod.remove() {
                                db::dd(&self.dbc.lock().unwrap(), deck.id).unwrap();
                                self.settings.dd(deck.id);
                            };
                        }
                    }
                    _ => {}
                }
                self.mode = self.mode_p;
            }
            Screen::Settings => {
                if let Some(sv) = &mut self.settings_view {
                    match sv.handle_input(c) {
                        views::ViewExit::Save(changes) => {
                            if self.mode_p == Screen::MainMenu {
                                self.settings.change(changes, None);
                                self.mode = Screen::MainMenu
                            } else {
                                //TODO: Should this call into the deck view? One source of truth for Deck ID?
                                self.settings.change(changes, Some(self.did));
                                self.deck_view.as_mut().unwrap().ucf();
                                self.mode_p = Screen::Settings;
                                self.mode = Screen::DeckView;
                            }
                        }
                        views::ViewExit::Hold => {}
                        views::ViewExit::Cancel => {
                            if self.mode_p == Screen::DeckView {
                                self.mode_p = Screen::Settings;
                                self.mode = Screen::DeckView;
                            } else {
                                self.mode = Screen::MainMenu
                            }
                        }
                        ViewExit::NewDeck(_) => {} //impossible
                    }
                }
            }
            Screen::DeckView => {
                let a = self
                    .deck_view
                    .as_mut()
                    .unwrap()
                    .handle_input(c, &self.dbc.lock().unwrap());
                match a {
                    DeckViewExit::Hold => {}
                    DeckViewExit::MainMenu => self.mode = Screen::MainMenu,
                    DeckViewExit::Stats => self.mode = Screen::DeckStat,
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
                self.create_deck_view = Some(CreateDeckView::new());
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
                self.init_deck_view();
            }
            Some(_) => {}
            None => self.quit = true,
        }
    }

    fn init_create_view(&mut self) {}

    fn init_deck_view(&mut self) {
        self.did = self.settings.rr();

        self.deck_view = Some(DeckView::new(
            self.did,
            &self.dbc.lock().unwrap(),
            self.settings.rds(self.did),
        ));
        self.mode = Screen::DeckView;
    }

    fn init_settings(&mut self, odid: Option<i32>) {
        let sv = match odid {
            Some(did) => views::SettingsView::new(
                self.settings.get_tags_deck(did),
                self.settings.rdf(Some(did)),
                self.settings.rso(Some(did)),
                String::from("Deck Settings"),
                None,
            ),
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
        self.stod.init(&self.dbc.lock().unwrap());
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

    //TODO: See about refactoring away?
    pub fn get_main_cards(&self) -> Vec<CardStat> {
        db::rvmcfd(&self.dbc.lock().unwrap(), self.did).unwrap()
    }

    //TODO: See about refactoring into DeckView?
    pub fn generate_dss_info(&self) -> DeckStatInfo {
        let mut dsi = DeckStatInfo::default();
        let vc = db::rvmcfd(&self.dbc.lock().unwrap(), self.did).unwrap();
        let types = vec![
            "Legendary",
            "Land",
            "Creature",
            "Planeswalker",
            "Enchantment",
            "Instant",
            "Sorcery",
            "Artifact",
        ];
        let mut hm_type: HashMap<String, u64> = HashMap::new();
        let mut hm_cmc: HashMap<String, u64> = HashMap::new();
        let mut hm_tag: HashMap<String, u64> = HashMap::new();

        for s in types.clone() {
            hm_type.insert(String::from(s), 0);
        }
        for num in 0..7 {
            hm_cmc.insert((num as u8).to_string(), 0);
        }
        hm_cmc.insert("7+".to_string(), 0);

        for c in vc {
            for t in c.types.clone().split(" ") {
                match t {
                    "Legendary" => {
                        let a: u64 = hm_type.get(&String::from("Legendary")).unwrap() + 1;
                        hm_type.insert(String::from("Legendary"), a);
                    }
                    "Land" => {
                        let a: u64 = hm_type.get(&String::from("Land")).unwrap() + 1;
                        hm_type.insert(String::from("Land"), a);
                    }
                    "Creature" => {
                        let a: u64 = hm_type.get(&String::from("Creature")).unwrap() + 1;
                        hm_type.insert(String::from("Creature"), a);
                    }
                    "Planeswalker" => {
                        let a: u64 = hm_type.get(&String::from("Planeswalker")).unwrap() + 1;
                        hm_type.insert(String::from("Planeswalker"), a);
                    }
                    "Enchantment" => {
                        let a: u64 = hm_type.get(&String::from("Enchantment")).unwrap() + 1;
                        hm_type.insert(String::from("Enchantment"), a);
                    }
                    "Instant" => {
                        let a: u64 = hm_type.get(&String::from("Instant")).unwrap() + 1;
                        hm_type.insert(String::from("Instant"), a);
                    }
                    "Sorcery" => {
                        let a: u64 = hm_type.get(&String::from("Sorcery")).unwrap() + 1;
                        hm_type.insert(String::from("Sorcery"), a);
                    }
                    "Artifact" => {
                        let a: u64 = hm_type.get(&String::from("Artifact")).unwrap() + 1;
                        hm_type.insert(String::from("Artifact"), a);
                    }
                    _ => {}
                }
            }

            match c.cmc {
                0 => {
                    if !c.types.contains("Land") {
                        let a: u64 = hm_cmc.get(&String::from("0")).unwrap() + 1;
                        hm_cmc.insert(String::from("0"), a);
                    }
                }
                1 => {
                    let a: u64 = hm_cmc.get(&String::from("1")).unwrap() + 1;
                    hm_cmc.insert(String::from("1"), a);
                }
                2 => {
                    let a: u64 = hm_cmc.get(&String::from("2")).unwrap() + 1;
                    hm_cmc.insert(String::from("2"), a);
                }
                3 => {
                    let a: u64 = hm_cmc.get(&String::from("3")).unwrap() + 1;
                    hm_cmc.insert(String::from("3"), a);
                }
                4 => {
                    let a: u64 = hm_cmc.get(&String::from("4")).unwrap() + 1;
                    hm_cmc.insert(String::from("4"), a);
                }
                5 => {
                    let a: u64 = hm_cmc.get(&String::from("5")).unwrap() + 1;
                    hm_cmc.insert(String::from("5"), a);
                }
                6 => {
                    let a: u64 = hm_cmc.get(&String::from("6")).unwrap() + 1;
                    hm_cmc.insert(String::from("6"), a);
                }
                _ => {
                    let a: u64 = hm_cmc.get(&String::from("7+")).unwrap() + 1;
                    hm_cmc.insert(String::from("7+"), a);
                }
            }

            for tag in c.tags {
                if let Some(v) = hm_tag.get_mut(&tag) {
                    let a: u64 = v.checked_add(1).unwrap();
                    hm_tag.insert(tag, a);
                } else {
                    hm_tag.insert(tag, 1);
                }
            }

            dsi.price_data.push((c.name.clone(), c.price));
        }

        hm_tag.remove_entry(&String::from("main"));

        for (k, v) in hm_cmc {
            dsi.cmc_data.push((k.clone(), v));
        }
        for (k, v) in hm_type {
            dsi.type_data.push((k.clone(), v));
        }
        for (k, v) in hm_tag {
            dsi.tag_data.push((k.clone(), v));
        }

        dsi.cmc_data.sort_by(|a, b| a.0.cmp(&b.0));
        dsi.price_data
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        dsi.type_data.sort_by(|a, b| a.0.cmp(&b.0));
        dsi.tag_data.sort_by(|a, b| {
            let o = b.1.cmp(&a.1);
            if o == Ordering::Equal {
                return a.0.cmp(&b.0);
            }
            o
        });

        dsi
    }

    pub fn generate_deck_table(&mut self) -> Table {
        self.stod.rdt()
    }

    pub fn render(&self, frame: &mut tui::Frame<CrosstermBackend<std::io::Stdout>>) {
        match self.mode {
            Screen::DeckView => self.deck_view.as_ref().unwrap().render(frame),
            Screen::Settings => self.settings_view.as_ref().unwrap().render(frame),
            Screen::MakeDeck => self.create_deck_view.as_ref().unwrap().render(frame),
            _ => todo!(),
        }
    }
}

fn generate_deckstat_managroup<'a>(vcs: &'a Vec<CardStat>) -> List<'a> {
    let mut cc = HashMap::new();
    let mut total = 0;
    for cs in vcs {
        for ch in cs.mana_cost.chars() {
            if ['W', 'U', 'B', 'R', 'G', 'C', 'X'].contains(&ch) {
                if let Some(i) = cc.get_mut(&ch) {
                    *i += 1;
                } else {
                    cc.insert(ch, 1);
                }
                total += 1;
            }
        }
    }
    let vli: Vec<ListItem> = cc
        .drain()
        .sorted()
        .map(|(symbol, amount)| {
            let percentage: f64 = 100.0 * (amount as f64) / (total as f64);
            ListItem::new(format!("{}: {} ({:.1}%)", symbol, amount, percentage))
        })
        .collect();
    List::new(vli).block(Block::default().title("Mana Colors").borders(Borders::ALL))
}

fn generate_deckstat_recommendations<'a>(vcs: &'a Vec<CardStat>) -> Paragraph<'a> {
    let mut nonlands = 0;
    let mut total_cmc: u16 = 0;
    let mut cc = HashMap::new();
    let mut recs = Vec::new();

    for cs in vcs {
        total_cmc += cs.cmc as u16;
        if let Some(i) = cc.get_mut(&cs.cmc) {
            *i += 1;
        } else {
            cc.insert(cs.cmc, 1);
        }
        // TODO: check for modal cards/double-facing.
        if !cs.types.contains("Land") {
            nonlands += 1;
        }
        // TODO: Add check for legalities after adding them to CardStat
    }

    if nonlands < 60 {
        recs.push(format!(
            "Only {} nonland cards in deck! Consider adding more.",
            nonlands
        ));
    } else if nonlands > 70 {
        recs.push(format!(
            "{} nonland cards in deck! Is that too many?",
            nonlands
        ));
    } else {
        recs.push(format!("{} nonland cards in deck.", nonlands));
    }
    let avg_cmc: f64 = ((total_cmc as f64) / (nonlands as f64)).into();
    if avg_cmc > 4.0 {
        recs.push(format!("Average mana cost {:.2}. Seems high.", avg_cmc));
    } else if avg_cmc < 3.0 {
        recs.push(format!("Average mana cost {:.2}. Seems low.", avg_cmc));
    } else {
        recs.push(format!("Average mana cost {:.2}.", avg_cmc));
    }

    Paragraph::new(recs.join("\n"))
        .block(Block::default().title("Deck Notes").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn generate_deckstat_manacurve<'a>(vcs: &'a Vec<CardStat>) -> Vec<(&str, u64)> {
    let mut hm_cmc: HashMap<&str, u64> = HashMap::new();
    hm_cmc.insert("0", 0);
    hm_cmc.insert("1", 0);
    hm_cmc.insert("2", 0);
    hm_cmc.insert("3", 0);
    hm_cmc.insert("4", 0);
    hm_cmc.insert("5", 0);
    hm_cmc.insert("6", 0);
    hm_cmc.insert("7+", 0);

    for cs in vcs {
        match cs.cmc {
            0 => {
                if !cs.types.contains("Land") {
                    let a = hm_cmc.get_mut("0").unwrap();
                    *a += 1;
                }
            }
            1 => {
                let a = hm_cmc.get_mut("1").unwrap();
                *a += 1;
            }
            2 => {
                let a = hm_cmc.get_mut("2").unwrap();
                *a += 1;
            }
            3 => {
                let a = hm_cmc.get_mut("3").unwrap();
                *a += 1;
            }
            4 => {
                let a = hm_cmc.get_mut("4").unwrap();
                *a += 1;
            }
            5 => {
                let a = hm_cmc.get_mut("5").unwrap();
                *a += 1;
            }
            6 => {
                let a = hm_cmc.get_mut("6").unwrap();
                *a += 1;
            }
            _ => {
                let a = hm_cmc.get_mut("7+").unwrap();
                *a += 1;
            }
        }
    }

    let data: Vec<(&str, u64)> = hm_cmc
        .drain()
        .sorted_by_key(|x| x.0)
        .map(|(k, v)| (k, v))
        .collect();
    data
}

fn generate_deckstat_barchart<'a>(title: &'a str, vd: &'a Vec<(&'a str, u64)>) -> BarChart<'a> {
    BarChart::default()
        .block(Block::default().title(title).borders(Borders::ALL))
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::White))
        .value_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(Color::Cyan))
        .data(vd.as_slice().clone())
}

fn draw<'a>(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    let _a = terminal.draw(|f| {
        let chunks = match state.mode {
            Screen::MainMenu | Screen::OpenDeck => Layout::default()
                .constraints([Constraint::Percentage(100)])
                .split(f.size()),
            Screen::MakeDeck => Vec::new(),
            Screen::Error(_) => Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size()),
            Screen::Settings => Vec::new(),
            Screen::DeckStat => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(4)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(f.size());

                let mut top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[0]);

                let mut bottom_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[1]);

                top_chunks.append(&mut bottom_chunks);
                top_chunks
            }
            Screen::DeckView => Vec::new(),
        };

        match state.mode {
            Screen::MainMenu => {
                let list = List::new(state.slmm.rvli())
                    .block(Block::default().title("Main Menu").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    );

                f.render_stateful_widget(list, chunks[0], &mut state.slmm.state.clone());
            }
            Screen::OpenDeck => {
                let mut ts = state.stod.state.clone();
                let table = state.generate_deck_table();

                f.render_stateful_widget(table, chunks[0], &mut ts);
            }
            Screen::Settings => state.render(f),
            Screen::Error(s) => {
                let (title, mut message) = s.split_once("\n").unwrap();
                let s = message.replace("{DECK}", state.stod.get().unwrap().name.as_str());
                if title == "Confirm Deletion" {
                    message = s.as_str();
                }
                let err_message = Paragraph::new(message)
                    .block(Block::default().borders(Borders::ALL).title(title));
                let area = centered_rect(60, f.size());
                f.render_widget(err_message, area);
            }
            Screen::MakeDeck => {
                state.render(f)
            }
            Screen::DeckStat => {
                let dsi = state.generate_dss_info();
                let vcs = state.get_main_cards();

                let cmc_data: Vec<(&str, u64)> =
                    dsi.cmc_data.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                let type_data: Vec<(&str, u64)> = dsi
                    .type_data
                    .iter()
                    .map(|(k, v)| (k.as_str(), *v))
                    .collect();
                let tag_data: Vec<ListItem> = dsi
                    .tag_data
                    .iter()
                    .map(|(k, v)| ListItem::new(format!("{}: {}", k, v)))
                    .collect();

                let dss = DeckStatScreen::from(&cmc_data, &dsi.price_data, &type_data, tag_data);
                let mg = generate_deckstat_managroup(&vcs);
                let recs = generate_deckstat_recommendations(&vcs);
                let mc = generate_deckstat_manacurve(&vcs);
                let mcc = generate_deckstat_barchart("Mana Values", &mc);

                f.render_widget(mcc, chunks[0]);
                f.render_widget(dss.prices, chunks[1]);
                f.render_widget(mg, chunks[2]);
                f.render_widget(dss.type_breakdown, chunks[3]);
                f.render_widget(dss.tag_list, chunks[4]);
                f.render_widget(recs, chunks[5]);
            }
            Screen::DeckView => state.render(f),
        }
    })?;
    Ok(())
}

fn centered_rect(percent_x: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(r.height / 2 - 3),
                Constraint::Length(5),
                Constraint::Length(r.height / 2 - 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
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
        draw(&mut terminal, &mut state)?;

        if state.mode != Screen::DeckStat {
            if let Event::Key(KeyEvent { code, .. }) = read()? {
                let _a = state.handle_input(code);
                if state.mode == Screen::DeckStat {
                    let did = state.did.clone();
                    let arc = Arc::clone(&state.dbc);
                    thread::spawn(move || {
                        db::ucfd(&arc, did).unwrap();
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
