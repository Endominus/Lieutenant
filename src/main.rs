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
extern crate peg;
extern crate lazy_static;

use lieutenant::db;
use lieutenant::ui;
use lieutenant::util;

use rusqlite::Connection;
use std::{fs::File, io::BufRead};
use std::io::BufReader;
// use std::collections::HashMap;
// use std::path::Path;
// use std::time::{Duration, Instant};

use clap::{App, Arg, SubCommand};
use anyhow::Result;

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
        .version("0.6")
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
            let mut p = std::env::current_exe().unwrap();
            p.pop();
            p.push("settings.toml");
            if !p.exists() {
                panic!("Cannot find the settings file. Are you sure it's in the same directory as the executable?");
            }

            let mut config = util::Settings::new(&p).unwrap();
            config.add_tag(1, String::from("mill"));
            config.add_tag(3, String::from("test"));
            // println!("{}", config.to_toml());

            let deck = db::rdfdid(&conn, 1).unwrap();
            let s = String::from("az");
            let cf = db::CardFilter::from(&deck, &s, config.get_default_filter(1));
            println!("Cardfilter produces: \n{}", cf.make_filter(false, config.get_sort_order(1)));

        }
        _ => { let _a = run(Command::Draw); }
    }
}
