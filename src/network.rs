use reqwest::blocking::get;
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

pub fn rvs() -> Result<Vec<Set>> {
    let mut sets = Vec::new();
    let url = "https://mtgjson.com/api/v5/SetList.json".to_string();
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

pub fn rvjc(set_code: &str) -> Result<Vec<JsonCard>> {
    let mut vjc = Vec::new();
    let url = format!("https://mtgjson.com/api/v5/{}.json", set_code);
    let res: serde_json::Value = get(&url)?.json().unwrap();
    let cards = match &res["data"]["cards"] {
        serde_json::Value::Array(i) => { i }
        _ => { panic!(); }
    };
    
    for value in cards {
        let d: JsonCard = serde_json::from_value(value.clone()).unwrap();
        vjc.push(d);
    }
    Ok(vjc)
}

pub fn rcostfcn(cn: &str, prev: Option<f64>) -> Result<f64> {
    let api = format!("https://api.scryfall.com/cards/search?q=name=%22{}%22", cn);
    let res_json: Value = get(api).unwrap().json().unwrap();
    let mut price = 0.0;
    if let Value::Array(vc) = &res_json["data"] {
        for c in vc {
            if let Value::Object(c) = c {
                if c["name"].as_str().unwrap() == cn {
                    let a = &c["prices"]["usd"];
                    match a {
                        Value::Null => {
                            let a = &c["prices"]["usd_foil"];
                            if let Value::String(s) = a {
                                price = s.parse().unwrap();
                            }
                        },
                        Value::String(s) => price = s.parse().unwrap(),
                        _ => {}
                    }
                }
            }
        }
    };

    if let Some(prev) = prev {
        if price > prev*1.5 || price == 0.0 {
            return rextcostfcn(cn)
        }
    }

    Ok(price)
}

pub fn rextcostfcn(cn: &str) -> Result<f64> {
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
