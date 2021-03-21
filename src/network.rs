

// use reqwest::Client;
// use reqwest::Error;
use reqwest::blocking::get;
use std::{thread, time};
use serde::Deserialize;
// use serde::de::{self, Deserialize};
// use reqwest::Response;
// use std::io::Read;

// use self::serde_json::Value;
use anyhow::Result;
use crate::db::Set;
use crate::Card;


fn jsonarray_to_vec(an: &str, c: &json::JsonValue) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    for i in c[an].members() {
        results.push(i.to_string());
    }
    
    results
}
// #[derive(Deserialize, Debug)]
// pub struct Sets {
//     pub sets: Vec<crate::db::Set>,
// }

// #[derive(Deserialize)]
// pub struct Cards {
//     cards: Vec<crate::Card>,
//     // meta: i8
// }

pub fn retrieve_card_by_name(name: String) -> Result<Vec<Card>> {
    let url = format!("https://api.magicthegathering.io/v1/cards?name=\"{}\"", name);
    rvc(url, 1)
    // todo!()
}

pub fn rvs() -> Result<Vec<Set>> {
    println!("Retrieving all sets now...");

    let url = "https://api.magicthegathering.io/v1/sets";
    let response = get(url)?;

    // let sets = response.json::<Sets>()?;
    let sets = response.json::<Vec<Set>>()?;

    println!("Retrieved a total of {} sets.", sets.len());

    Ok(sets)
}

pub fn rcs(s: &crate::db::Set) -> Vec<Card> {
    let url = format!("https://api.magicthegathering.io/v1/cards?set={}", s.code);
    let c = rvc(url, 1).unwrap();

    c
    // todo!()
}

fn rvc(url: String, page: i8) -> Result<Vec<Card>> {
    let url = format!("{url}&page={page}", url = url, page = page);
    let res = get(&url)?;

    // let mut cards = res.json::<Cards>()?;
    let mut cards = res.json::<Vec<Card>>()?;

    
    if cards.len() == 100 {
        // println!("Found {}, going to next page", cards.len());
        thread::sleep(time::Duration::from_secs(1));
        cards.append(&mut rvc(url, page+1).unwrap());
    }
    Ok(cards)
    // todo!()
}
