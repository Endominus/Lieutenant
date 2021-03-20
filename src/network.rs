

// use reqwest::Client;
// use reqwest::Error;
// use reqwest::blocking::get;
// use std::{thread, time};
use serde::Deserialize;
// use serde::de::{self, Deserialize};
// use reqwest::Response;
// use std::io::Read;

// use self::serde_json::Value;
use anyhow::Result;


fn jsonarray_to_vec(an: &str, c: &json::JsonValue) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    for i in c[an].members() {
        results.push(i.to_string());
    }
    
    results
}
#[derive(Deserialize, Debug)]
pub struct Sets {
    pub sets: Vec<crate::db::Set>,
}

#[derive(Deserialize)]
pub struct Cards {
    cards: Vec<crate::Card>,
    // meta: i8
}

pub fn retrieve_card_by_name(name: String) -> Result<Vec<crate::Card>> {
    // let url = format!("https://api.magicthegathering.io/v1/cards?name=\"{}\"", name);
    // rc(url, 1)
    todo!()
}

pub fn rs() -> Result<Vec<crate::db::Set>> {
    // let url = "https://api.magicthegathering.io/v1/sets";
    // let mut response = reqwest::get(url)?;

    // let sets = response.json::<Sets>()?;

    // println!("{:?}", sets.sets);

    // Ok(sets.sets)
    todo!()
}

pub fn rcs(s: &crate::db::Set) -> Vec<crate::Card> {
    // let url = format!("https://api.magicthegathering.io/v1/cards?set={}", s.code);
    // let c = rc(url, 1).unwrap();

    // c
    todo!()
}

async fn rc(url: String, page: i8) -> Result<Vec<crate::Card>> {
    // let url = format!("{url}&page={page}", url = url, page = page);
    // let mut res = reqwest::get(&url).await?;

    // let mut cards = res.json::<Cards>().await?;

    
    // if cards.cards.len() == 100 {
    //     // println!("Found {}, going to next page", cards.len());
    //     thread::sleep(time::Duration::from_secs(2));
    //     cards.cards.append(&mut rc(url, page+1).unwrap());
    // }
    // Ok(cards.cards)
    todo!()
}
