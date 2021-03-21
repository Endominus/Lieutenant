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
// extern crate json;

// mod res;
// use res::{Command, run};

// use serde::{Deserialize};
use clap::{App, Arg, SubCommand};
use anyhow::Result;
use serde::Deserialize;
// use serde::de::Deserialize;

mod network;
mod db;
mod ui;

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

fn zero() -> String { String::from("0") }

pub struct Deck {
    pub name: String,
    pub commander: Card,
    pub id: i32,
}

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
            let a = db::rvcn(card, -1)?;
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
    // println!("{:?}", args);
    let matches = App::new("Lieutenant")
        .version("0.3")
        .about("Helps you manage your commander decks")
        .author("Endominus")
        .arg(
            Arg::with_name("update")
                .help("Updates the database")
                .short("u")
                .long("update")
        )
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
                )
        ])
        //TODO: Add deck import command.
        // .subcommand(
        // )
        .get_matches();

    if matches.is_present("update"){
        let _a = run(Command::FullPull);
        if let Err(e) = run(Command::FullPull) {
            println!("Error: {}", e);
        }
    }

    if let Some(ref matches) = matches.subcommand_matches("get") {
        // Safe to use unwrap() because of the required() option
        println!("Getting cards with name: {}", matches.value_of("input").unwrap());
        let _a = run(Command::RetrieveCard(matches.value_of("input").unwrap().to_string()));
    }

    if let Some(ref matches) = matches.subcommand_matches("import") {
        println!("Inserting all cards from {} into deck with ID {}", 
            matches.value_of("filename").unwrap(), 
            matches.value_of("deck_id").unwrap());
            let _a = run(Command::ImportCards(
                matches.value_of("deck_id").unwrap().parse().unwrap(),
                matches.value_of("filename").unwrap().to_string()));
    }

    // let a = lieutenant::run(lieutenant::Command::RetrieveCard("Avacyn, Guardian Angel".to_string()));
    // println!("{:?}", a);
    let _a = run(Command::Draw);
}
