// extern crate cursive;

// use cursive::Cursive;
// use cursive::views::{Dialog, TextView};

// use std::io::{self, Write};
// use std::collections::HashMap;

extern crate json;
extern crate reqwest;


use std::io::Read;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut res = reqwest::get("https://api.magicthegathering.io/v1/cards?name=Avacyn")?;
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let parsed = json::parse(&body).unwrap();

    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    // println!("Body:\n{}", parsed["cards"]);

    let mut seen: Vec<&json::JsonValue> = Vec::new();
    

    for card in parsed["cards"].members() {
        if seen.contains(&&card["name"]) {
            continue;
        }
        seen.push(&card["name"]);
        println!("Name: {}", card["name"]);
        println!("Supertypes: {}", card["supertypes"]);
        println!("Types: {}", card["types"]);
        println!("Subtypes: {}", card["subtypes"]);
        println!("Text: {}", card["text"]);
        println!("Cmc: {}", card["cmc"]);
        println!("ColorIdentity: {}", card["colorIdentity"]);
        println!("Names: {}", card["names"]);
        println!("Mana Cost: {}\n", card["manaCost"]);
    }
    Ok(())
}

// // #[macro_use]
// extern crate ureq;
// extern crate json;

// fn main() {

//     // sync post request of some json.
//     let mut request = ureq::get("https://api.magicthegathering.io/v1/cards");
//     let response = request.query("name", "Griselbrand").call();
//     let text = response.into_json().unwrap();
//     let null = &text["cards"][101];
//     let mut c = 0;
//     let mut card = &text["cards"][c];
//     while (card != null) & (c < 20) {
//         println!("Name: {}", card["name"]);
//         println!("Mana Cost: {}", card["manaCost"]);
//         println!("Text: {}", card["text"]);
//         c += 1;
//         card = &text["cards"][c];
//     }
//     // for card in text["cards"].iter() {

//     // }
// }