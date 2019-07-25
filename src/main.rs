extern crate cursive;

// use cursive::Cursive;
// use cursive::views::{Dialog, TextView};

// use std::io::{self, Write};
// use std::collections::HashMap;

// use std::io::Read;
// extern crate reqwest;

// fn main() -> Result<(), Box<std::error::Error>> {
//     let mut res = reqwest::get("https://api.magicthegathering.io/v1/cards?name=Griselbrand")?;
//     let mut body = String::new();
//     res.read_to_string(&mut body)?;

//     println!("Status: {}", res.status());
//     println!("Headers:\n{:#?}", res.headers());
//     println!("Body:\n{}", body);
//     Ok(())
// }

// #[macro_use]
extern crate ureq;
extern crate json;

fn main() {

    // sync post request of some json.
    let mut request = ureq::get("https://api.magicthegathering.io/v1/cards");
    let response = request.query("name", "Griselbrand").call();
    let text = response.into_json().unwrap();
    let null = &text["cards"][101];
    let mut c = 0;
    let mut card = &text["cards"][c];
    while (card != null) & (c < 20) {
        println!("Name: {}", card["name"]);
        println!("Mana Cost: {}", card["manaCost"]);
        println!("Text: {}", card["text"]);
        c += 1;
        card = &text["cards"][c];
    }
    // for card in text["cards"].iter() {

    // }
}