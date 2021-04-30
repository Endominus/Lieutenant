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
extern crate csv;

use lieutenant::db;
use lieutenant::ui;
use lieutenant::util;

use rusqlite::Connection;
use std::{fs::File, io::BufRead};
use std::io::BufReader;
use std::path::PathBuf;
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
    ImportCards(String, Vec<String>, PathBuf),
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
            let a = db::rcfn(&conn, &card, None)?;
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
        Command::ImportCards(deck_name, commanders, filename) => {
            let p = util::get_local_file("lieutenant.db");
            let conn = Connection::open(p).unwrap();

            let mut cards = Vec::new();
            if let Some(ext) = filename.extension() {
                match ext.to_str().unwrap() {
                    "txt" => {
                        let file =  File::open(&filename).unwrap();
                        let buf = BufReader::new(file);
                        for a in buf.lines() {
                            let ic = db::ImportCard { name: a.unwrap(), tags: None };
                            cards.push(ic);
                        }
                        db::import_deck(&conn, deck_name, commanders, cards)?;
                    }
                    "csv" => {
                        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_path(&filename)?;
                        for result in rdr.records() {
                            let record = result?;
                            match record.deserialize::<db::ImportCard>(None) {
                                Ok(p) => {println!("{:?}", p);}
                                Err(e) => {println!("Error found {:?}", e);}
                            }
                        }
                    }
                    _ => {
                        println!("Wrong extension; imports will only work from txt and csv files.")
                    }
                }
            };

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
                .about("Imports cards from a csv or text file into a deck")
                .arg(
                    Arg::with_name("deck")
                        .help("Desired name of the deck")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("filename")
                       .help("Name of the file to import from")
                       .index(2)
                       .required(true),
                )
                .arg(
                    Arg::with_name("commander")
                        .short("com")
                        .help("Name of the commander(s)")
                        .multiple(true)
                        .index(3)
                        .required(false)
                        .takes_value(true)
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
            // let (primary, secondary) = match sub_m.value_of("primary_commander") {
            //     Some(ps) => { 
            //         match sub_m.value_of("secondary_commander") {
            //             Some(ss) => { (ps.to_string(), ss.to_string())  }
            //             None => { (ps.to_string(), String::new())  }
            //         }
            //     }
            //     None => { (String::new(), String::new()) }
            // };
            let commanders: Vec<_> = match sub_m.values_of("commander") {
                Some(vs) => { vs.map(|s| s.to_string()).collect() }
                None => { Vec::new() }
            };
            println!("Creating deck {} with commander {:?}. Adding all cards from {}.",
                sub_m.value_of("deck").unwrap(),
                commanders,
                sub_m.value_of("filename").unwrap()); 
            let _a = run(Command::ImportCards(
                    sub_m.value_of("deck").unwrap().to_string(),
                    commanders,
                    PathBuf::from(sub_m.value_of("filename").unwrap())));
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
