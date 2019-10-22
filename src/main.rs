#![allow(dead_code)]

extern crate reqwest;
extern crate rusqlite;
extern crate json;
extern crate clap;

use clap::{App, Arg, SubCommand};

fn main() {
    // println!("{:?}", args);
    let matches = App::new("Lieutenant")
        .version("0.2")
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
        if let Err(e) = lieutenant::run(lieutenant::Command::FullPull) {
            println!("Error: {}", e);
        }
    }

    if let Some(ref matches) = matches.subcommand_matches("get") {
        // Safe to use unwrap() because of the required() option
        println!("Getting cards with name: {}", matches.value_of("input").unwrap());
        let _a = lieutenant::run(lieutenant::Command::RetrieveCard(matches.value_of("input").unwrap().to_string()));
    }

    if let Some(ref matches) = matches.subcommand_matches("import") {
        println!("Inserting all cards from {} into deck with ID {}", 
            matches.value_of("filename").unwrap(), 
            matches.value_of("deck_id").unwrap());
            let _a = lieutenant::run(lieutenant::Command::ImportCards(
                matches.value_of("deck_id").unwrap().parse().unwrap(),
                matches.value_of("filename").unwrap().to_string()));
    }

    // let a = lieutenant::run(lieutenant::Command::RetrieveCard("Avacyn, Guardian Angel".to_string()));
    // println!("{:?}", a);
    let _a = lieutenant::run(lieutenant::Command::Draw);
}
