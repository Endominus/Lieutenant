// use crate::util::Card;
// mod db;

// use std::{thread, time};
// use reqwest::Response;
use reqwest::blocking::get;
// use reqwest::get;
use anyhow::Result;
use serde_json::Value;
use crate::db::{JsonCard, Set};


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

// pub fn retrieve_card_by_name(name: String) -> Result<Vec<JsonCard>> {
//     let url = format!("https://api.magicthegathering.io/v1/cards?name=\"{}\"", name);
//     rvc(url, 1)
//     // todo!()
// }

// pub fn rvs() -> Result<Vec<Set>> {
//     println!("Retrieving all sets now...");

//     let url = "https://api.magicthegathering.io/v1/sets";
//     let response = get(url)?;

//     // let sets = response.json::<Sets>()?;
//     let sets = response.json::<Vec<Set>>()?;

//     println!("Retrieved a total of {} sets.", sets.len());

//     Ok(sets)
// }

// pub fn rcs(s: &crate::db::Set) -> Vec<JsonCard> {
//     let url = format!("https://api.magicthegathering.io/v1/cards?set={}&legality=Commander", s.code);
//     let c = match rvc(url, 1) {
//         Ok(vc) => { vc }
//         Err(_) => { Vec::new() }
//     };

//     c
//     // todo!()
// }

pub fn rvs() -> Result<Vec<Set>> {
    let mut sets = Vec::new();
    let url = format!("https://mtgjson.com/api/v5/SetList.json");
    let res: serde_json::Value = get(&url)?.json().unwrap();
    let map =  match &res["data"] {
        serde_json::Value::Array(i) => { i }
        _ => { panic!(); }
    };
    println!("Found {} sets. Filtering...", map.len());
    let allowed_types = Vec::from(["expansion", "core", "commander", "draft_innovation"]);
    for value in map {
        let d: Set = serde_json::from_value(value.clone()).unwrap();
        if allowed_types.contains(&d.set_type.as_str()) { sets.push(d); }
    }
    println!("{} sets are commander-legal.", sets.len());

    Ok(sets)
}

pub fn rvjc(set_code: &String) -> Result<Vec<JsonCard>> {
    let mut vjc = Vec::new();
    let url = format!("https://mtgjson.com/api/v5/{}.json", set_code);
    let res: serde_json::Value = get(&url)?.json().unwrap();
    let cards = match &res["data"]["cards"] {
        serde_json::Value::Array(i) => { i }
        _ => { panic!(); }
    };
    // let cards = res
    for value in cards {
        let d: JsonCard = serde_json::from_value(value.clone()).unwrap();
        vjc.push(d);
    }
    Ok(vjc)
}

// fn rvc(url: String, page: i8) -> Result<Vec<JsonCard>> {
//     let url = format!("{url}&page={page}", url = url, page = page);
//     let res = get(&url)?;

//     // let mut cards = res.json::<Cards>()?;
//     let mut cards = res.json::<Vec<JsonCard>>()?;

    
//     if cards.len() == 100 {
//         // println!("Found {}, going to next page", cards.len());
//         thread::sleep(time::Duration::from_secs(1));
//         cards.append(&mut rvc(url, page+1).unwrap());
//     }
    
//     Ok(cards)
//     // todo!()
// }

pub fn rcostfcn(cn: &String, prev: Option<f64>) -> Result<f64> {
    let api = format!("https://api.scryfall.com/cards/search?q=name=%22{}%22", cn);
    // let res = get(api).await?.text().await?;
    // let res_json: Value = serde_json::from_str(res.as_str())?;
    let res_json: Value = get(api).unwrap().json().unwrap();
    // println!("{:?}", res_json);
    let price: f64 = match &res_json["data"] {
        Value::Array(vc) => {
            let mut r = 0.0;
            for c in vc {
                match c {
                    Value::Object(c) => {
                        if c["name"].as_str().unwrap() == cn {
                            // println!("{:?}", c["prices"]["usd"]);
                            r = c["prices"]["usd"].as_str().unwrap_or("0.0").parse().unwrap();
                        }
                    }
                    _ => { panic!(); }
                }
            }
            r
        }
        Value::Object(_) => { panic!(); }
        _ => { 0.0 }
    };

    if let Some(prev) = prev {
        if price > prev*1.5 {
            return rextcostfcn(cn)
        }
    }

    Ok(price)
}

pub fn rextcostfcn(cn: &String) -> Result<f64> {
    let api = format!("https://api.scryfall.com/cards/search?q=name=%22{}%22", cn);
    let res_json: Value = get(api).unwrap().json().unwrap();
    let mut res_list = Value::default();
    if let Value::Array(vv) = &res_json["data"] {
        for v in vv {
            if let Value::Object(o) = v {
                if o["name"].as_str().unwrap() == cn {
                    let api = o["prints_search_uri"].as_str().unwrap();
                    res_list = get(api).unwrap().json().unwrap();
                }
            }
        }
    }

    let mut vp: Vec<f64> = Vec::new();

    if let Value::Array(vv) = &res_list["data"] {
        for v in vv {
            if let Value::Object(o) = v {
                let r = o["prices"]["usd"].as_str().unwrap_or("invalid").parse();
                if let Ok(p) = r {
                    vp.push(p);
                }
            }
        }
    }

    vp.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = vp.get(vp.len()/2).unwrap();

    Ok(*median)
}
