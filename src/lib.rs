extern crate rusqlite;
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, Result};


#[derive(Debug)]
struct Card {
    name: String,
    types: Vec<String>,
    subtypes: Vec<String>,
    text: String,
    cmc: i8,
    mana_cost: String,
    color_identity: Vec<char>,
    related_cards: Vec<String>
}

impl Card {
    // fn new(name String, types)
}