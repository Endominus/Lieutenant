extern crate reqwest;
extern crate rusqlite;
extern crate json;
extern crate cursive;
extern crate clap;

use clap::{App, Arg};
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
        .subcommand(
            App::new("get") 
                .about("Gets a card from the database")
                .arg(
                    Arg::with_name("input")
                        .help("card to get")
                        .index(1)
                        .required(true),
                ),
        )
        .get_matches();

    if matches.is_present("update"){
        if let Err(e) = lieutenant::run(lieutenant::Command::FullPull) {
            println!("Error: {}", e);
        }
    }

    if let Some(ref matches) = matches.subcommand_matches("get") {
        // Safe to use unwrap() because of the required() option
        println!("Getting cards with name: {}", matches.value_of("input").unwrap());
        lieutenant::run(lieutenant::Command::RetrieveCard(matches.value_of("input").unwrap().to_string()));
    }

    // let a = lieutenant::run(lieutenant::Command::RetrieveCard("Avacyn, Guardian Angel".to_string()));
    // println!("{:?}", a);
    
}
