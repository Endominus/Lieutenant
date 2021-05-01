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
extern crate self_update;

use lieutenant::db;
use lieutenant::ui;
use lieutenant::util;

use std::{fs::File, path::PathBuf, io::{BufReader, BufRead}};
use rusqlite::Connection;
use clap::{App, Arg, SubCommand};
use anyhow::Result;
use self_update::cargo_crate_version;

pub enum Command {
    RetrieveCardOnline(String),
    RetrieveCard(String),
    FullPull,
    Update,
    Draw,
    ImportDeck(String, Vec<String>, PathBuf),
    ExportDeck(i32, Option<PathBuf>),
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
            //TODO: update card database as well
        },
        Command::Draw => { 
            let _a = ui::run();
        },
        Command::ImportDeck(deck_name, commanders, filename) => {
            let p = util::get_local_file("lieutenant.db", true);
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
                    }
                    _ => {
                        println!("Wrong extension; imports will only work from txt and csv files.")
                    }
                }

                db::import_deck(&conn, deck_name, commanders, cards)?;
            };

            // Ok(())
        }
        Command::ExportDeck(did, path) => {
            let p = util::get_local_file("lieutenant.db", false);
            let conn = Connection::open(p).unwrap();
            let cards = db::rvicfdid(&conn, did).unwrap();
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
                .about("Updates the application and card database."),
            SubCommand::with_name("debug")
                .about("For testing various features as developed."),
            SubCommand::with_name("export")
                .about("Exports a deck from a given deck id. If no output file is given, the csv will be generated in the same directory as the executable.")
                .arg(
                    Arg::with_name("deck_id")
                        .help("ID of the deck to export. Can be seen in the Open Deck screen.")
                        .index(1)
                        .required(true)
                )
                .arg(
                    Arg::with_name("out_file")
                        .help("Output file. Optional.")
                        .index(2)   
                        .required(false)
                )
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
            let commanders: Vec<_> = match sub_m.values_of("commander") {
                Some(vs) => { vs.map(|s| s.to_string()).collect() }
                None => { Vec::new() }
            };
            println!("Creating deck {} with commander {:?}. Adding all cards from {}.",
                sub_m.value_of("deck").unwrap(),
                commanders,
                sub_m.value_of("filename").unwrap()); 
            let _a = run(Command::ImportDeck(
                    sub_m.value_of("deck").unwrap().to_string(),
                    commanders,
                    PathBuf::from(sub_m.value_of("filename").unwrap())));
        }
        ("export", Some(sub_m)) => {
            let did: i32 = sub_m.value_of("deck_id").unwrap().parse().unwrap();
            let p = match sub_m.value_of("out_file") {
                Some(s) => { Some(PathBuf::from(s)) }
                None => { None }
            };
            run(Command::ExportDeck(did, p)).unwrap();
        }
        ("update", Some(_sub_m)) => {
            println!("Updating the application and database.");
            run(Command::Update).unwrap();
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
