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
        .get_matches();

    if matches.is_present("update"){
        if let Err(e) = lieutenant::run(lieutenant::Command::FullPull) {
            println!("Error: {}", e);
        }
    }

    let a = lieutenant::run(lieutenant::Command::RetrieveCard("Avacyn, Guardian Angel".to_string()));
    println!("{:?}", a);
    
}
