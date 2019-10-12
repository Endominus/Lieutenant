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
        let color_identity: Vec<String> = jsonarray_to_vec("colorIdentity", c);
        
        // println!("{}", c["name"]);        
        
        cards.push(
            crate::Card::new(
                c["name"].to_string(),
                types,
                supertypes,
                subtypes,
                c["text"].to_string(), 
                c["cmc"].as_i8().unwrap()/10, 
                c["manaCost"].to_string(), 
                color_identity, 
                names, 
                c["layout"].to_string()))
    }
    
    Ok(cards)
}

pub fn full_pull() {
    //TODO: Implement
}
