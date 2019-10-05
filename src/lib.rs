extern crate rusqlite;
extern crate reqwest;

use rusqlite::types::ToSql;
use rusqlite::{params, Connection, Result};

use std::error::Error;

use reqwest::Client;
use reqwest::Response;

#[derive(Debug)]
pub struct Card {
    pub name: String,
    pub types: Vec<String>,
    pub subtypes: Vec<String>,
    pub text: String,
    pub cmc: i8,
    pub mana_cost: String,
    pub color_identity: Vec<char>,
    pub related_cards: Vec<String>
}

impl Card {
    fn new(
        name: String, 
        types: Vec<String>, 
        subtypes: Vec<String>,
        text: String,
        cmc: i8,
        mana_cost: String,
        color_identity: Vec<char>,
        related_cards: Vec<String>) -> Card {
            Card {
                name,
                types,
                subtypes,
                text,
                cmc,
                mana_cost,
                color_identity,
                related_cards
            }
        }
}

fn run() {
    
}

mod Online {
    fn retrieve_card_by_name(name: String) -> Result<super::Card, &'static str> {
        let mut res = reqwest::get("https://api.magicthegathering.io/v1/cards?name=Avacyn").unwrap();
        let mut body = String::new();
        res.text().unwrap();
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
        // Ok(())
        panic!()
    }
}