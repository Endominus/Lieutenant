#![allow(dead_code)]
#![feature(derive_default_enum)]
mod network;
mod db;
mod ui;
mod util;

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
extern crate lazy_static;
extern crate csv;
extern crate self_update;
// #[macro_use]
extern crate pest_derive;


use chrono::Datelike;
use crate::db::CardFilter;
use crate::network::rvjc;
use crate::util::*;

use std::{fs::File, path::PathBuf, io::{BufReader, BufRead}};
use rusqlite::Connection;
use clap::{App, arg};
use anyhow::Result;
use self_update::cargo_crate_version;
use std::time::Instant;

pub enum Command {
    RetrieveCard(String),
    Update,
    Draw,
    ImportDeck(String, Vec<String>, PathBuf),
    ExportDeck(i32, Option<PathBuf>),
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::RetrieveCard(card) => {
            let conn = Connection::open("lieutenant.db")?;
            let a = db::rcfn(&conn, &card, None)?;
            println!("{:?}", a);
        },
        Command::Update => {
            let status = self_update::backends::github::Update::configure()
                .repo_owner("Endominus")
                .repo_name("Lieutenant")
                .bin_name("lieutenant")
                .show_download_progress(true)
                .current_version(cargo_crate_version!())
                .build()?
                .update()?;
            println!("Updated to version {}!", status.version());

            let now = Instant::now();
            let sets = network::rvs().unwrap();
            let p = util::get_local_file("lieutenant.db", true);
            let conn = Connection::open(p).unwrap();
            db::updatedb(&conn, sets).unwrap();
            println!("Imported cards in {} ms.", now.elapsed().as_millis());
        },
        Command::Draw => { 
            let _a = ui::run();
        },
        Command::ImportDeck(deck_name, commanders, filename) => {
            let p = util::get_local_file("lieutenant.db", true);
            let conn = Connection::open(p).unwrap();
            let p = util::get_local_file("settings.toml", true);
            let file_settings = FileSettings::new(&p).unwrap();
            let mut settings = Settings::from(file_settings);

            let mut cards = Vec::new();
            let mut tags = Vec::new();

            if let Some(ext) = filename.extension() {
                match ext.to_str().unwrap() {
                    "txt" => {
                        let file =  File::open(&filename).unwrap();
                        let buf = BufReader::new(file);
                        for a in buf.lines() {
                            let ic = db::ImportCard { name: a.unwrap(), tags: None };
                            cards.push(ic);
                        }
                    }
                    "csv" => {
                        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_path(&filename)?;
                        for result in rdr.records() {
                            let record = result?;
                            match record.deserialize::<db::ImportCard>(None) {
                                Ok(p) => { cards.push(p); }
                                Err(e) => {
                                    // The user may have forgotten to put a comma at the end of a line.
                                    match e.kind() {
                                        csv::ErrorKind::Deserialize { pos: _ , err } => {
                                            if let csv::DeserializeErrorKind::Message(s) = err.kind() {
                                                if err.field() == None 
                                                && s == &String::from("invalid length 1, expected struct ImportCard with 2 elements") {
                                                    // println!("Problem with the card: {:?}", record.get(0).unwrap());
                                                    let ic = db::ImportCard { name: String::from(record.get(0).unwrap()), tags: None };
                                                    cards.push(ic);

                                                }
                                            };
                                        }
                                        _ => { println!("Error found in csv file: {}", e); }
                                    }
                                }
                            }
                        }
                        for ic in &cards {
                            if let Some(s) = &ic.tags {
                                let vs = s.split('|');
                                for tag in vs {
                                    if !tags.contains(&String::from(tag)) && !tag.is_empty() {
                                        tags.push(String::from(tag));
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        println!("Wrong extension; imports will only work from txt and csv files.")
                    }
                }
                
                let did = db::import_deck(&conn, deck_name, commanders, cards)?;
                settings.id(did);
                for tag in tags { settings.it(Some(did), tag); }
                std::fs::write(p, settings.to_toml()).unwrap();
            };
        }
        Command::ExportDeck(did, path) => {
            let p = util::get_local_file("lieutenant.db", false);
            let conn = Connection::open(p).unwrap();
            let deck = db::rdfdid(&conn, did).expect("Deck could not be retrieved. Ensure Deck ID is correct.");
            let mut cards = db::rvicfdid(&conn, did).unwrap();
            let c = deck.commander;
            let i = cards.iter().position(|ic| ic.name == c.name).unwrap();
            let ic = cards.remove(i);
            cards.insert(0, ic);
            if let Some(c) = deck.commander2 {
                let i = cards.iter().position(|ic| ic.name == c.name).unwrap();
                let ic = cards.remove(i);
                cards.insert(1, ic);
            }

            let p = match path {
                Some(p) => { p }
                None => {
                    let deck = db::rdfdid(&conn, did).unwrap();
                    util::get_local_file(format!("{}.csv", deck.name).as_str(), false)
                }
            };

            let mut wtr = csv::Writer::from_path(p).unwrap();
            wtr.write_record(&["Card Name","Tags"]).unwrap();
            for card in cards {
                let tags = match card.tags { Some(s) => {s} None => {String::new()}};
                wtr.write_record(&[card.name, tags]).unwrap();

            }
        }
    }

    Ok(())
}


fn main() {
    let matches = App::new("Lieutenant")
        .version(cargo_crate_version!())
        .about("Helps you manage your commander decks")
        .author("Endominus")
        .subcommand(
            App::new("import")
            .about("Imports cards from a csv or text file into a deck")
            .args(&[
                arg!(<deckname> "Desired name of the deck"), //TODO: make this optional
                arg!(<filename> "Source file to import"),
                arg!([commander] "Name of commander (if not first row of the deck)"),
            ])
        ).subcommand(
            App::new("update")
            .about("Updates the application and card database.")
        ).subcommand(
            App::new("debug")
            .about("For testing various features as developed.")
            .arg(arg!(<module> "Specific part of the program to be tested."))
        ).subcommand(
            App::new("export")
            .about("Exports a deck from a given deck id. If no output file is given, the csv will be generated in the same directory as the executable.")
            .args(&[
                arg!(<deck_id> "ID of the deck to export. Can be seen in the Open Deck screen."),
                arg!([file] "Output file. Optional."),
            ])
        )
        .get_matches();


    match matches.subcommand() {
        Some(("import", sub_m)) => {
            let commanders: Vec<_> = match sub_m.values_of("commander") {
                Some(vs) => { vs.map(|s| s.to_string()).collect() }
                None => { Vec::new() }
            };
            println!("Creating deck {} with commander {:?}. Adding all cards from {}.",
                sub_m.value_of("deckname").unwrap(),
                commanders,
                sub_m.value_of("filename").unwrap()); 
            let _a = run(Command::ImportDeck(
                    sub_m.value_of("deckname").unwrap().to_string(),
                    commanders,
                    PathBuf::from(sub_m.value_of("filename").unwrap())));
        }
        Some(("export", sub_m)) => {
            let did: i32 = sub_m.value_of("deck_id").unwrap().parse().unwrap();
            let p = match sub_m.value_of("file") {
                Some(s) => { Some(PathBuf::from(s)) }
                None => { None }
            };
            run(Command::ExportDeck(did, p)).unwrap();
        }
        Some(("update", _sub_m)) => {
            println!("Updating the application and database.");
            run(Command::Update).unwrap();
        }
        Some(("debug", sub_m)) => {
            match sub_m.value_of("module").unwrap() {
                "rcfn" => { let _a = debug_rcfn(); },
                "rvjc" => { let _a = debug_rvjc(); },
                "parser" => { let _a = debug_parse_args(); },
                "filter" => { let _a = debug_rvcfcf(); },
                "settings" => { let _a = debug_settings(); },
                "network" => { let _a = debug_network(); },
                _ => {},
            }
        }
        _ => { let _a = run(Command::Draw); }
    }
}


fn debug_rvjc() -> Result<()> {
    let cards = rvjc(&String::from("STX"))?;
    println!("First card is: {}", cards[0].name);
    Ok(())
}

fn debug_parse_args() -> Result<()> {
    let p = util::get_local_file("lieutenant.db", false);
    let conn = Connection::open(p).unwrap();
    db::add_regexp_function(&conn).unwrap();
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.push("settings.toml");
    if !p.exists() {
        panic!("Cannot find the settings file. Are you sure it's in the same directory as the executable?");
    }

    // let config = util::Settings::new(&p).unwrap();
    let deck = db::rdfdid(&conn, 3).unwrap();
    let cf = CardFilter::from(deck.id, &deck.color, util::DefaultFilter::Name, util::SortOrder::NameAsc);

    let s = String::from("na:elesh|norn");
    println!("For \"{}\", Cardfilter produces: \n{}", &s, cf.make_query(false, &s));
    let s = String::from("tag:main");
    println!("For \"{}\", Cardfilter produces: \n{}", &s, cf.make_query(false, &s));
    let s = String::from("tag:main ty:a sort:+cmc");
    println!("For \"{}\", Cardfilter produces: \n{}", &s, cf.make_query(false, &s));

    Ok(())
}

fn debug_rvcfcf() -> Result<()> {
    // let p = util::get_local_file("lieutenant.db", false);
    // let conn = Connection::open(p).unwrap();
    // db::add_regexp_function(&conn).unwrap();
    // let mut p = std::env::current_exe().unwrap();
    // p.pop();
    // p.push("settings.toml");
    // if !p.exists() {
    //     panic!("Cannot find the settings file. Are you sure it's in the same directory as the executable?");
    // }

    // let config = util::Settings::new(&p).unwrap();

    // let deck = db::rdfdid(&conn, 1).unwrap();
    // let s = String::from("te:\"enters the\"");
    // let cf = db::CardFilter::from(&deck, &s, config.get_default_filter(1));
    // match db::rvcfcf(&conn, cf, false, config.get_sort_order(1)) {
    //     Ok(vc) => { println!("Found {} cards.", vc.len()); },
    //     Err(res) => { println!("Error was: {}", res); },
    // }




    Ok(())
}

fn debug_rcfn() {
    let p = util::get_local_file("lieutenant.db", false);
    let conn = Connection::open(p).unwrap();
    db::add_regexp_function(&conn).unwrap();

    let a = db::rcfndid(
        &conn, 
        &"Anafenza, Kin-Tree Spirit".to_string(), 
        1).unwrap();
    
    println!("{:?}", a);
}

fn debug_settings() -> Result<()> {
    // println!("{}", config.to_toml());

    Ok(())
}

fn debug_network() -> Result<()> {
    println!("Quick price is: {:?}", network::rcostfcn(&String::from("Sol Ring"), None));
    println!("Detailed price is: {:?}", network::rextcostfcn(&String::from("Sol Ring")));

    let sets = network::rvs().unwrap();
    let now = chrono::Utc::now();
    let date = format!("{}-{}-{}", now.year(), now.month(), now.day());

    for set in sets {
        if set.date > date {
            println!("Set {} should not be here.", set.name)
        }
    }

    Ok(())
}