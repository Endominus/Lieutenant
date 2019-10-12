use reqwest::Client;
use reqwest::Response;

pub fn retrieve_card_by_name(name: String) -> Result<Vec<crate::Card>, &'static str> {
    let url = format!("https://api.magicthegathering.io/v1/cards?name={}", name);
    let mut res = reqwest::get(&url).unwrap();
    let body = res.text().unwrap();
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

pub fn full_pull() {
    //TODO: Implement
}
