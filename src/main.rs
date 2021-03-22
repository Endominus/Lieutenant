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
extern crate lazy_static;

// mod res;
// use res::{Command, run};

// use serde::{Deserialize};

use std::collections::HashMap;
use std::sync::RwLock;
use config::Config;
use clap::{App, Arg, SubCommand};
use anyhow::Result;
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
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub name: String,
    pub supertypes: Vec<String>,
    pub types: Vec<String>,
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub text: String,
    pub cmc: f64,
    #[serde(default = "zero")]
    pub mana_cost: String,
    pub color_identity: Vec<String>,
    #[serde(rename = "names")]
    #[serde(default)]
    pub related_cards: Vec<String>,
    #[serde(default)]
    pub power: String,
    #[serde(default)]
    pub toughness: String,
    pub layout : String,
}

impl ToString for Card { fn to_string(& self) -> String { self.name.clone() } }

fn zero() -> String { String::from("0") }

pub struct Deck {
    pub name: String,
    pub commander: Card,
    pub id: i32,
}

impl ToString for Deck { fn to_string(& self) -> String { self.name.clone() } }

impl Card {
    pub fn ri(&self) -> Vec<String> {
        let t = self.text.split("\n");
        let types = format!("{} {} - {}", 
            self.supertypes.join(" "),
            self.types.join(" "),
            self.subtypes.join(" "));

        let mut v = vec![
            self.name.clone(),
            self.mana_cost.clone(),
            types
        ];

        for l in t {
            v.push(l.to_string());
        }

        if self.power.len() > 0 {
            v.push(format!("{}/{}", self.power, self.toughness));
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
    ImportCards(usize, String),
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::RetrieveCardOnline(card) => {
            let a = network::retrieve_card_by_name(card)?;
            for card in a {
                println!("{:?}", card);
            }

            // Ok(())
        },
        Command::RetrieveCard(card) => {
            let a = db::rvcfname(card, -1)?;
            for card in a {
                println!("{:?}", card);
            }

            // Ok(())
        },
        Command::FullPull => {
            db::create_db()?;
            db::full_pull()?;
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
        Command::ImportCards(did, filename) => {
            db::import_deck(filename, did)?;
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
            SubCommand::with_name("import")
                .about("Imports cards from a file into a deck")
                .arg(
                    Arg::with_name("deck_id")
                        .help("Deck ID number")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("filename")
                       .help("Name of the file to import from")
                       .index(2)
                       .required(true),
                ),
            SubCommand::with_name("update")
                .about("Updates the card database with any new cards added"),
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
        ("import", Some(sub_m)) => {
            println!("Inserting all cards from {} into deck with ID {}", 
            sub_m.value_of("filename").unwrap(), 
            sub_m.value_of("deck_id").unwrap());
            // let _a = run(Command::ImportCards(
                //     sub_m.value_of("deck_id").unwrap().parse().unwrap(),
                //     sub_m.value_of("filename").unwrap().to_string()));
            }
        ("update", Some(_sub_m)) => {
            println!("Updating the database");
            // let _a = run(Command::FullPull);
            // if let Err(e) = run(Command::FullPull) {
            //     println!("Error: {}", e);
            // }
        }
        _ => { let _a = run(Command::Draw); }
    }
}
