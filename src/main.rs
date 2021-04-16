#![allow(dead_code)]
extern crate reqwest;
extern crate rusqlite;
extern crate json;
extern crate clap;
extern crate anyhow;
#[cfg(feature = "serde_derive")] 
extern crate serde;
extern crate crossterm;
extern crate tui;
extern crate serde_json;
// #[macro_use]
extern crate peg;
#[macro_use]
extern crate lazy_static;

// use lieutenant::network::rcostfcn;
use lieutenant::db;
use lieutenant::ui;
// use lieutenant::db::CardFilter;

use rusqlite::Connection;
use std::{collections::HashMap, fs::File, io::BufRead};
use std::io::BufReader;
// use std::collections::HashMap;
// use std::path::Path;
// use std::time::{Duration, Instant};


use serde::Deserialize;

use std::sync::RwLock;
use config::{Config, ConfigError};
use clap::{App, Arg, SubCommand};
use anyhow::Result;

#[derive(Debug, Deserialize)]
struct SettingsGroup {
    tags: Option<Vec<String>>,
    ordering: Option<String>,
    default_filter: Option<String>
}

#[derive(Debug, Deserialize)]
struct Settings {
    global: SettingsGroup,
    decks: HashMap<usize, SettingsGroup>
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::default();
        s.merge(config::File::with_name("settings.toml")).unwrap();

        s.try_into()
    }

    pub fn get_tags(&self) -> Vec<String> {
        self.global.tags.as_ref().unwrap().clone()
    }

    pub fn get_tags_deck(&self, deck: usize) -> Vec<String> {
        let mut r = Vec::new();
        if let Some(s) = self.decks.get(&deck) {
            if let Some(t) = &s.tags {
                r.append(&mut t.clone());
            };
        };
        r.append(&mut self.global.tags.as_ref().unwrap().clone());
        r
    }
}

lazy_static! {
    static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::new().unwrap());
}

pub enum Command {
    RetrieveCardOnline(String),
    RetrieveCard(String),
    FullPull,
    UpdateDB,
    Draw,
    ImportCards(String, String, String),
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::RetrieveCardOnline(_card) => {
            // let a = db::rpfdc(&card);
            // println!("Card price is: {}", a)
            // for card in a {
            //     println!("{:?}", card);
            // }

            // Ok(())
        },
        Command::RetrieveCard(card) => {
            // let cf = CardFilter::from(-1, &card);
            let conn = Connection::open("lieutenant.db")?;
            let a = db::rcfn(&conn, &card)?;
            // for card in a {
            println!("{:?}", a);
            // }

            // Ok(())
        },
        Command::FullPull => {
            // db::create_db()?;
            // db::full_pull()?;
            // network::rs();
            // println!("{:?}", a);
            // Ok(())
        },
        Command::UpdateDB => {unimplemented!()},
        Command::Draw => { 
            //TODO Make general
            let _a = ui::run();
            // Ok(()) 
        },
        Command::ImportCards(deck_name, com_name, filename) => {
            let conn = Connection::open("lieutenant.db")?;
            let file =  File::open(filename).unwrap();
            let buf = BufReader::new(file);
            let mut cards = Vec::new();
            for a in buf.lines() {
                cards.push(a.unwrap());
            }

            db::import_deck(&conn, deck_name, com_name, cards)?;
            // Ok(())
        }
    }

    Ok(())
}


fn main() {
    let matches = App::new("Lieutenant")
        .version("0.5")
        .about("Helps you manage your commander decks")
        .author("Endominus")
        .subcommands( vec![
            SubCommand::with_name("get") 
                .about("Gets a card from the database")
                .arg(
                    Arg::with_name("input")
                        .help("card to get")
                        .index(1)
                        .required(true),
                ),
            SubCommand::with_name("price")
                .about("Retrieves the price of a given card")
                .arg(
                    Arg::with_name("input")
                        .help("card to get")
                        .index(1)
                        .required(true),
                ),
            SubCommand::with_name("import")
                .about("Imports cards from a file into a deck")
                .arg(
                    Arg::with_name("deck_name")
                        .help("Desired name of the deck")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("com_name")
                        .help("Name of the commander")
                        .index(2)
                        .required(true)
                )
                .arg(
                    Arg::with_name("filename")
                       .help("Name of the file to import from")
                       .index(3)
                       .required(true),
                ),
            SubCommand::with_name("update")
                .about("Updates the card database with any new cards added"),
            SubCommand::with_name("debug")
                    .about("For testing various features as developed."),
        ])
        .get_matches();


    match matches.subcommand() {
        ("get", Some(sub_m)) => {
            println!("Getting cards with name: {}", sub_m.value_of("input").unwrap());
            let _a = run(Command::RetrieveCard(sub_m.value_of("input").unwrap().to_string()));
        }
        ("price", Some(sub_m)) => {
            let s = sub_m.value_of("input").unwrap();
            println!("Getting cards with name: {}", s);
            let _a = run(Command::RetrieveCardOnline(s.to_string()));

        }
        ("import", Some(sub_m)) => {
            println!("Creating deck {} with commander {}. Adding all cards from {}.",
                sub_m.value_of("deck_name").unwrap(),
                sub_m.value_of("com_name").unwrap(),
                sub_m.value_of("filename").unwrap()); 
            let _a = run(Command::ImportCards(
                    sub_m.value_of("deck_name").unwrap().to_string(),
                    sub_m.value_of("com_name").unwrap().to_string(),
                    sub_m.value_of("filename").unwrap().to_string()));
            }
        ("update", Some(_sub_m)) => {
            println!("Updating the database");
            // let _a = run(Command::FullPull);
            // if let Err(e) = run(Command::FullPull) {
            //     println!("Error: {}", e);
            // }
        }
        ("debug", Some(_sub_m)) => {
            let conn = Connection::open("lieutenant.db").unwrap();
            db::add_regexp_function(&conn).unwrap();
            let deck = db::rdfdid(&conn, 1).unwrap();
            let omni = String::new();
            let cf = db::CardFilter::from(&deck, &omni);
            println!("Cardfilter produces: {}", cf.make_filter(true));



            // let mut s = Config::default();
            // s.merge(config::File::with_name("settings.toml")).unwrap();
            // println!("{:?}", s);


    // TODO if settings file doesn't exist, create it with default values.

            // println!("{:?}", SETTINGS.read().unwrap().clone().try_into::<HashMap<String, Settings>>().unwrap());
            // println!("{:?}", SETTINGS.read().unwrap().get_tags());
            // println!("{:?}", SETTINGS.read().unwrap().get_tags_deck(1));
            // println!("{:?}", SETTINGS.read().unwrap().get_tags_deck(2));
            // println!("{:?}", SETTINGS.read().unwrap().get_tags_deck(3));
            // SETTINGS.write().unwrap().set("recent", 1).unwrap();
            // println!("{:?}", SETTINGS.read().unwrap().clone().try_into::<HashMap<String, String>>().unwrap());

            // let a = network::rcs(& db::Set { code: String::from("TPH1"), name: String::from("Theros Path of Heroes") });
            // let cf = db::CardFilter::new(1).text(String::from("ana"));
            // println!("Cardfilter produces: {}", cf.make_filter());
            // println!("{:?}", CardFilter::parse_omni("ana"));
            // println!("{:?}", CardFilter::parse_omni("n:\"kor sky\""));
            // println!("{:?}", CardFilter::parse_omni("name:\" of \""));
            // println!("{:?}", CardFilter::parse_omni("text:\"draw a card\""));
            // println!("{:?}", CardFilter::parse_omni("text:\"+1\""));
            // println!("{:?}", CardFilter::parse_omni("text:\"+1\" n:aja"));
            // println!("{:?}", CardFilter::parse_omni("text:\"+1\" n:aja ty:creature"));
            // println!("{:?}", CardFilter::parse_omni("te:lifelink"));
            // println!("{:?}", CardFilter::parse_omni("te:\"draw a card\" n:Ajani"));
            // println!("{:?}", CardFilter::parse_omni("color:c"));
            // println!("{:?}", CardFilter::parse_omni("c:w name:blue"));
            // println!("{:?}", CardFilter::parse_omni("c:wb"));
            // println!("{:?}", CardFilter::parse_omni("color:w|b"));
            // println!("{:?}", CardFilter::parse_omni("color:b|g/w"));
            // println!("{:?}", CardFilter::parse_omni("type:creature"));
            // println!("{:?}", CardFilter::parse_omni("ty:legendary+sorcery"));
            // println!("{:?}", CardFilter::parse_omni("ty:legendary+creature/sorcery+tribal/instant name:\"how are you\""));
            // println!("{:?}", CardFilter::parse_omni("ty:c"));
            // println!("{:?}", CardFilter::parse_omni("ty:coward"));
            // println!("{:?}", CardFilter::parse_omni("ty:instant te:draw ajani"));
            // println!("{:?}", CardFilter::parse_omni("cmc:0-4"));
            // println!("{:?}", CardFilter::parse_omni("cmc:-4"));
            // println!("{:?}", CardFilter::parse_omni("cmc:4-"));
            // println!("{:?}", CardFilter::parse_omni("cmc:<10"));
            // println!("{:?}", CardFilter::parse_omni("cmc:>10"));
            // println!("{:?}", CardFilter::parse_omni("ci:wb"));
            // println!("{:?}", CardFilter::parse_omni("ci:wr"));
            // println!("{:?}", CardFilter::parse_omni("coloridentity:w/b"));
            // println!("{:?}", CardFilter::parse_omni("color_identity:b|g|w"));
            // println!("{:?}", CardFilter::parse_omni("p:0-4"));
            // println!("{:?}", CardFilter::parse_omni("p:-4"));
            // println!("{:?}", CardFilter::parse_omni("power:4-"));
            // println!("{:?}", CardFilter::parse_omni("power:-"));
            // println!("{:?}", CardFilter::parse_omni("power:"));
            // println!("{:?}", CardFilter::parse_omni("n:"));
            // println!("{:?}", CardFilter::parse_omni("n:\"\""));
            // println!("{:?}", CardFilter::parse_omni("power:"));
            // println!("{:?}", CardFilter::parse_omni("te:"));
            // println!("{:?}", CardFilter::parse_omni("color:"));
            // println!("{:?}", CardFilter::parse_omni("c:"));
            // println!("{:?}", CardFilter::parse_omni("ty:"));
            // println!("{:?}", CardFilter::parse_omni("cmc:"));
            // println!("{:?}", CardFilter::parse_omni("coloridentity:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni(""));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("n:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("n:\"\""));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("power:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("te:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("color:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("c:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("ty:"));
            // // assert_eq!(HashMap::new(), CardFilter::parse_omni("cmc:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("coloridentity:"));
            
            // let omni = String::from("n:\"kor sky\" ty:artifact cmc:>4 te:w|");
            // let omni = String::from("ty:artifact cmc:>4 color:w|");
            // let omni = String::from("ty:artifact ci:wr cmc:2-");
            // let cf = CardFilter::from(5, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("ty: cmc:>4");
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("cmc:>4 tag:");
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("cmc:>4 color_identity:");
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("text:\"you control can\'t\" c:r|g");
            // let omni = String::from("cmc:<4 tag:ramp"); //tags REGEXP '\|?ramp(?:$|\|)'
            // let omni = String::from("cmc:<4 ci:c"); //color_identity REGEXP '^[^WUBRG]*$'
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // println!("{:?}", db::rvcfcf(&conn, cf, false));//.iter().map(|f| f.to_string()).collect::<Vec<String>>());

            // let s = "WHERE regexp('.*ozi.*', name)";
            // let s = "WHERE name REGEXP \'.*ozi.*\' AND mana_cost REGEXP \'R\'";
            // let s = "WHERE cards.name LIKE \'%ana%\'";
            // let s = "%ana%";
            // println!("{:?}", db::db_test(s).unwrap().len());

            // let now = Instant::now();
            // let file = File::open("AtomicCards.json").unwrap();
            // let reader = BufReader::new(file);
            // let a: serde_json::Value = serde_json::from_reader(reader).unwrap();
            // println!("Imported cards in {} s.", now.elapsed().as_secs());
            // let now = Instant::now();
            // let _iresult = db::initdb(&conn);
            // let (a, b) = db::ivcfjsmap(&conn, a).unwrap();
            // println!("Inserted {} rows with {} failures in {} ms.", a, b, now.elapsed().as_millis());
            // println!("{}", a["data"]["Chalice of Life // Chalice of Death"]);
            // let c: NewCard = serde_json::from_value(
            //     a["data"]["Chalice of Life // Chalice of Death"].clone())
            //     .unwrap();
            // println!("{:?}", c)
            
            // println!("{} cards in {} s", vc.len(), now.elapsed().as_secs());
            // let now = Instant::now();
            // let a = db::create_new_database();
            // println!("{:?}: Cards added to database in {} s", a, now.elapsed().as_secs());

            // let future = async move {
            //     let a = rcostfcn(&"Raging Goblin".to_string()).await;
            //     println!("{:?}", a)
            // };

            // let res = tokio::runtime::Builder::new()
            //     .basic_scheduler()
            //     .enable_all()
            //     .build()
            //     .unwrap()
            //     .block_on(future);
            // res
            // let _a = db::ucfd(&conn, 2);


        }
        _ => { let _a = run(Command::Draw); }
    }
}
