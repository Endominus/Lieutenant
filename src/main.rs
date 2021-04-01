#![allow(unused_imports)]
#![feature(str_split_once)]

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
#[macro_use]
extern crate peg;
#[macro_use]
extern crate lazy_static;

use json::object::Object;
use network::rcostfcn;
use rusqlite::Connection;
use std::{collections::HashMap, fs::File, io::BufRead};
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};
// #[macro_use]

// mod res;
// use res::{Command, run};

// use serde::{Deserialize};

// use std::collections::HashMap;
use std::sync::RwLock;
use config::Config;
use clap::{App, Arg, SubCommand};
use anyhow::Result;
use db::{CardFilter};
use serde::Deserialize;
// use serde::de::Deserialize;

mod network;
mod db;
mod ui;

lazy_static! {
    static ref SETTINGS: RwLock<Config> = RwLock::new({
        let mut settings = Config::default();
        settings.merge(config::File::with_name("settings.toml")).unwrap();

        settings
    });
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Legalities {
    #[serde(default)]
    brawl: String,
    #[serde(default)]
    commander: String,
    #[serde(default)]
    duel: String,
    #[serde(default)]
    future: String,
    #[serde(default)]
    frontier: String,
    #[serde(default)]
    historic: String,
    #[serde(default)]
    legacy: String,
    #[serde(default)]
    modern: String,
    #[serde(default)]
    pauper: String,
    #[serde(default)]
    penny: String,
    #[serde(default)]
    pioneer: String,
    #[serde(default)]
    standard: String,
    #[serde(default)]
    vintage: String,
}

impl Legalities {
    fn to_vec(map: serde_json::Value) -> Vec<String> {
        let legalities = match map {
            serde_json::Value::Object(i) => { i }
            _ => { return Vec::new() }
        };

        legalities.keys().cloned().collect()
    }

    fn from(s: String) -> Legalities {
        let mut l = Legalities::default();

        if let Some(_) = s.find("commander") { l.commander = String::from("Allowed"); }

        l
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Relation {
    Single(String),
    Meld {face: String, transform: String },
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Layout {
    Adventure(char, String),
    Aftermath(char, String),
    Flip(char, String),
    Leveler,
    Meld(char, String, String),
    ModalDfc(char, String),
    Normal,
    Saga,
    Split(char, String),
    Transform(char, String),
}

//TODO: Add a JsonCard struct to facilitate import from json.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonCard {
    #[serde(rename = "convertedManaCost")]
    pub cmc: f64,
    pub color_identity: Vec<String>,
    pub legalities: Legalities,
    #[serde(default)]
    pub loyalty: String,
    #[serde(default = "zero")]
    pub mana_cost: String,
    pub name: String,
    #[serde(default)]
    pub power: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub toughness: String,
    #[serde(rename = "type")]
    pub types: String,
    pub layout : String,
    pub related_cards: Option<Relation>,
    pub side: Option<char>,
    //TODO: Add rarity and sets
}

impl JsonCard {
    pub fn convert(&self) -> Card { todo!(); }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub cmc: f64,
    pub color_identity: Vec<String>,
    pub legalities: Legalities,
    pub loyalty: String,
    pub mana_cost: String,
    pub name: String,
    pub power: String,
    pub tags: Vec<String>,
    pub text: String,
    pub toughness: String,
    pub types: String,
    pub lo: Layout,
    //TODO: Add rarity and sets
}

impl ToString for Card { fn to_string(& self) -> String { self.name.clone() } }

impl ToString for Legalities { 
    fn to_string(& self) -> String {
        let mut vs = Vec::new();
        let b = vec![String::default(), String::from("Banned")];

        if !b.contains(&self.brawl) { vs.push("brawl"); }
        if !b.contains(&self.commander) { vs.push("commander"); }
        if !b.contains(&self.modern) { vs.push("modern"); }
        if !b.contains(&self.standard) { vs.push("standard"); }

        vs.join("|")
    }
}

pub struct CardStat {
    pub cmc: u8,
    pub color_identity: Vec<String>,
    pub mana_cost: String,
    pub name: String,
    pub tags: Vec<String>,
    pub types: String,
    pub price: f64,
}

fn zero() -> String { String::from("0") }

pub struct Deck {
    pub name: String,
    pub commander: Card,
    pub id: i32,
}

impl ToString for Deck { fn to_string(& self) -> String { self.name.clone() } }

impl Card {
    pub fn ri(&self) -> Vec<String> {
        let mut v = vec![
            self.name.clone(),
            format!("{}, ({})", self.mana_cost, self.cmc),
            self.types.clone(),
            String::new(),
        ];
            
        let t = self.text.split("\n");
        for l in t {
            v.push(l.to_string());
        }

        if self.power.len() > 0 {
            v.push(format!("Power/Toughness: {}/{}", self.power, self.toughness));
        } else if self.loyalty.len() > 0 {
            v.push(format!("Loyalty: {}", self.loyalty.clone()));
        }

        v.push(String::new());

        match &self.lo {
            Layout::Adventure(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Also has Adventure: {}", rel)); } 
                    'b' => { v.push(format!("Adventure of: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::Aftermath(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Also has Aftermath: {}", rel)); } 
                    'b' => { v.push(format!("Aftermath of: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::Flip(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Also has Flip side: {}", rel)); } 
                    'b' => { v.push(format!("Flip side of: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::ModalDfc(_, rel) => { v.push(format!("You may instead cast: {}", rel)); }
            Layout::Split(_, rel) => { v.push(format!("You may instead cast: {}", rel)); }
            Layout::Transform(side, rel) => { 
                match side { 
                    'a' => { v.push(format!("Transforms into: {}", rel)); } 
                    'b' => { v.push(format!("Transforms from: {}", rel)); } 
                    _ => {} 
                }
            }
            Layout::Meld(side, face, meld) => { 
                match side { 
                    'a' => { v.push(format!("Melds with {} to form {}", face, meld)); } 
                    'b' => { v.push(format!("Melds from {} and {}", face, meld)); } 
                    _ => {} 
                }
            }
            _ => {}
        }
        
        v.push(String::new());


        if self.tags.len() > 0 {
            v.push(format!("Tags: {}", self.tags.join(" ")));
        }
        v
    }
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
        .version("0.3")
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

    // TODO if settings file doesn't exist, create it with default values.

    // println!("{:?}", SETTINGS.read().unwrap().clone().try_into::<HashMap<String, String>>().unwrap());
    // SETTINGS.write().unwrap().set("recent", 1).unwrap();
    // println!("{:?}", SETTINGS.read().unwrap().clone().try_into::<HashMap<String, String>>().unwrap());

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
            // let cf = CardFilter::from(2, & omni);
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
            let _a = db::ucfd(&conn, 2);


        }
        _ => { let _a = run(Command::Draw); }
    }
}
