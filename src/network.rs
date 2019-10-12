extern crate serde_json;
extern crate json;

// use reqwest::Client;
use reqwest::Error;
// use reqwest::Response;
// use std::io::Read;

// use self::serde_json::Value;

fn jsonarray_to_vec(an: &str, c: &json::JsonValue) -> Vec<String> {
   let mut results: Vec<String> = Vec::new();
   for i in c[an].members() {
       results.push(i.to_string());
   }

   results
}

pub fn retrieve_card_by_name(name: String) -> std::result::Result<Vec<crate::Card>, Error> {
    let url = format!("https://api.magicthegathering.io/v1/cards?name={}", name);
    let mut res = reqwest::get(&url)?;

    let mut cards: Vec<crate::Card> = Vec::new();

    let body = res.text()?;
    let parsed = json::parse(&body).unwrap();
    let json_cards = &parsed["cards"];

    let mut seen: Vec<&json::JsonValue> = Vec::new();

    for c in json_cards.members() {
        if seen.contains(&&c["name"]) {
            continue;
        }
        seen.push(&c["name"]);
        
        let types: Vec<String> = jsonarray_to_vec("types", c);
        let supertypes: Vec<String> = jsonarray_to_vec("supertypes", c);
        let subtypes: Vec<String> = jsonarray_to_vec("subtypes", c);
        let names: Vec<String> = jsonarray_to_vec("names", c);
        
        let mut color_identity: Vec<char> = Vec::new();
        for i in c["colorIdentity"].members() {
            color_identity.push(i.as_str()
                .unwrap().chars().next().unwrap());
        }
        
        println!("{}", c["name"]);

        
        
        cards.push(
            crate::Card::new(
                c["name"].to_string(),
                types,
                supertypes,
                subtypes,
                c["text"].to_string(), 
                c["cmc"].as_i8().unwrap(), 
                c["manaCost"].to_string(), 
                color_identity, 
                names, 
                c["layout"].to_string()))
    }
    
    // let v: Value = serde_json::from_str(&json_cards.to_string()).unwrap();
    // println!("{:?}", v[0]["supertypes"].as_array().unwrap());

    // let cards: Vec<crate::Card> = json_cards.json();

    // for card in parsed["cards"].members() {
    //     if seen.contains(&&card["name"]) {
    //         continue;
    //     }
    //     seen.push(&card["name"]);
    //     cards.push(
    //         crate::Card::new(
    //             card["name"].to_string(),
    //             card["types"],
    //             card["supertypes"], 
    //             card["subtypes"], 
    //             card["text"].to_string(), 
    //             card["cmc"].as_i8().unwrap(), 
    //             card["manaCost"].to_string(), 
    //             card["colorIdentity"], 
    //             card["names"], 
    //             card["layout"].to_string()))
    // }

        // println!("Name: {}", card["name"]);
        // println!("Supertypes: {}", card["supertypes"]);
        // println!("Types: {}", card["types"]);
        // println!("Subtypes: {}", card["subtypes"]);
        // println!("Text: {}", card["text"]);
        // println!("Cmc: {}", card["cmc"]);
        // println!("ColorIdentity: {}", card["colorIdentity"]);
        // println!("Names: {}", card["names"]);
        // println!("Mana Cost: {}\n", card["manaCost"]);
    Ok(cards)
}

pub fn full_pull() {
    //TODO: Implement
}
