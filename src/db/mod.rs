extern crate rusqlite;
extern crate regex;
extern crate tokio;

use crate::{CardStat, JsonCard, Layout, Legalities};
use self::rusqlite::{params, Connection, Result, NO_PARAMS, Error};
use crate::{Deck, Card, Relation};
use std::{collections::HashMap, convert::TryInto, fs};
use rusqlite::{Row, Statement, named_params};
use serde::Deserialize;
use regex::Regex;
use serde_json::Value;
use self::rusqlite::functions::FunctionFlags;
use std::sync::Arc;
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
use std::{thread, time};

use futures::{io::BufReader, prelude::*};
// use tokio::prelude::*;

// pub struct DbContext<'a> {
//     conn: Connection,
//     stmts: HashMap<&'a str, Statement<'a>>
// }

use crate::network::rcostfcn;

const DB_FILE: &str = "lieutenant.db";

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Set {
    pub code: String,
    name: String,
}

#[derive(Default)]
pub struct CardFilter<'a> {
    did: i32,
    fi: HashMap<&'a str, String>
}

impl<'a> CardFilter<'a> {
    pub fn new() -> CardFilter<'a> {
        // CardFilter {
        //     did,
        //     name: String::new(),
        //     text: String::new(),

        // }

        CardFilter::default()
    }

    pub fn from(did: i32, omni: &'a String) -> CardFilter<'a> {
        let mut cf = CardFilter::default();

        cf.did = did;
        if omni.len() > 0 { 
            cf.fi = CardFilter::parse_omni(omni.as_str()); 
        } else {
            cf.fi = HashMap::new();
        }

        cf
    }

    pub fn parse_omni(omni: &str) -> HashMap<&str, String> {
        let mut hm = HashMap::new();
        // hm.insert(String::from("test"), String::from("test"));

        // let omni = String::from(omni);

        peg::parser!{
            grammar omni_parser() for str {
                pub rule root(hm: &mut HashMap<&str, String>)
                = (fields(hm) / " "+)+ ![_]

                rule fields(hm: &mut HashMap<&str, String>) = (
                text(hm) 
                / tag(hm)
                / color(hm) 
                / ctype(hm) 
                / cmc(hm) 
                / color_identity(hm) 
                / power(hm) 
                / toughness(hm) 
                / sort(hm)
                / name(hm)
                )

                rule cmc(hm: &mut std::collections::HashMap<&str, String>)
                = "cmc:" value:$number_range() { hm.insert("cmc", String::from(value)); }
                rule tag(hm: &mut HashMap<&str, String>)
                = tag_alias() ":" value:text_group() ** and_separator() { hm.insert("tag", value.join("|")); }
                rule name(hm: &mut HashMap<&str, String>)
                = name_alias()? value:ss_values() { hm.insert("name", value); }
                rule text(hm: &mut HashMap<&str, String>)
                = text_alias() ":" value:text_group() ** or_separator() { if value[0] != String::default() { hm.insert("text", value.join("|")); }}
                rule sort(hm: &mut HashMap<&str, String>)
                = "sort:" value:$(['+' | '-'] ("name" / "cmc")) { hm.insert("sort", String::from(value)); }
                rule color(hm: &mut HashMap<&str, String>)
                = color_alias() ":" value:$(colors()+) ** or_separator() { if !value.is_empty() { hm.insert("color", value.join("|")); }}
                rule ctype(hm: &mut HashMap<&str, String>)
                = type_alias() ":" value:type_group() ** or_separator() { if value[0] != String::default() { hm.insert("type", value.join("|")); }}
                rule power(hm: &mut std::collections::HashMap<&str, String>)
                = power_alias() ":" value:$((['0'..='9']+)? "-"? (['0'..='9']+)?) { if value != "" { hm.insert("power", String::from(value)); }}
                rule toughness(hm: &mut std::collections::HashMap<&str, String>)
                = toughness_alias() ":" value:$((['0'..='9']+)? "-"? (['0'..='9']+)?) { if value != "" { hm.insert("toughness", String::from(value)); }}
                rule color_identity(hm: &mut HashMap<&str, String>)
                = color_identity_alias() ":" value:$(colors()+) ** or_separator() { if !value.is_empty() { hm.insert("color_identity", value.join("|")); }}

                rule ss_values() -> String
                = v:$(phrase() / word()) { String::from(v) }
                rule type_group() -> String
                = and_types:word() ** and_separator() { and_types.join("&") }
                rule text_group() -> String
                = and_text:$(word() / phrase()) ** and_separator() { and_text.join("&") }
                rule number_range() = ['-' | '>' | '<'] ['0'..='9']+ / ['0'..='9']+ "-"? (['0'..='9']+)?

                rule name_alias() = ("name:" / "n:")
                rule text_alias() = ("text" / "te")
                rule type_alias() = ("type" / "ty")
                rule color_alias() = ("color" / "c")
                rule power_alias() = ("power" / "p")
                rule toughness_alias() = ("toughness" / "t")
                rule color_identity_alias() = ("color_identity" / "coloridentity" / "ci")
                rule tag_alias() = ("tag" / "tags")

                rule word() -> String
                = s:$("!"? ['a'..='z' | '0'..='9' | '{' | '}' | '\'' | '.' | '_'| '\'']+) { String::from(s) }
                rule phrase() -> String
                =s:$("!"? "\"" (word() / " " / "+" / ":" / "/")+ "\"" ) {String::from(s) }
                // rule exp_types() -> String
                // = t:$types() { match t {
                //     "l" => String::from("legendary"),
                //     "e" => String::from("enchantment"),
                //     "p" => String::from("planeswalker"),
                //     "i" => String::from("instant"),
                //     "s" => String::from("sorcery"),
                //     "c" => String::from("creature"),
                //     "a" => String::from("artifact"),
                //     _ => String::from("ERROR"),
                // } }
                // =s:$("\"" [_]* "\"" ) {String::from(s) }

                rule colors() = ['c' | 'w' | 'u' | 'b' | 'g' | 'r']
                rule all_separator() = ['|' | '/' | '+' | '&']
                rule or_separator() = ['|' | '/' ]
                rule and_separator() = ['+' | '&' ]
                
            }
        }

        match omni_parser::root(omni, &mut hm) {
            Ok(_) => {}
            Err(_) => { 
                // println!("Attempted to run omniparser with incorrect arguments. Resultant hashmap:\n{:?}", hm); 
            }
        }

        // for (k, v) in &hm {
        //     if v.is_empty() {
        //         hm.remove(k);
        //     }
        // }

        // hm.
        
        hm
    }

    pub fn make_filter(&self, conn: &Connection, general: bool) -> String {
        let initial = match general {
            true => { 
                let com = rcomfdid(conn, self.did, false).unwrap();
                let mut colors = String::from("WUBRG");
                for ci in com.color_identity {
                    colors = colors.replace(&ci, "");
                }

                if let Ok(com) = rcomfdid(conn, self.did, true) {
                    for ci in com.color_identity {
                        colors = colors.replace(&ci, "");
                    }
                }
                format!("WHERE color_identity REGEXP \'^[^{}]*$\'", colors) 
            }
            false => { format!("
INNER JOIN deck_contents
ON cards.name = deck_contents.card_name
WHERE deck_contents.deck = {}", self.did) }
        };
        
        let mut order = String::from("ASC");
        let mut sort_on = String::from("name");
        let mut vs = Vec::from([initial]);

        for (key, value) in self.fi.clone() {
            match key {
                "name" => { vs.push(format!("AND (cards.name LIKE \"%{}%\")", value.trim_matches('\"'))); }
                "tag" => { 
                    vs.push(format!(r#"AND tags IS NOT NULL"#));
                    let tags = value.split("&");
                    for tag in tags {
                        vs.push(format!(r#"AND tags REGEXP '\|?{}(?:$|\|)'"#, tag));
                    }
                }
                "text" => { 
                    let tegs = value.split("|"); 
                    let mut vteg = Vec::new();
                    for teg in tegs {
                        let mut vf = Vec::new();
                        for mut te in teg.split('&') {
                            let include = match te.get(0..1) {
                                Some("!") => { te = te.get(1..).unwrap(); "NOT LIKE" }
                                Some(_) => { "LIKE" }
                                None => { continue; }
                            };
                            vf.push(format!("card_text {} \"%{}%\"", include, te.trim_matches('\"')));
                        }
                        if vf.len() > 0 {
                            vteg.push(format!("({})", vf.join(" AND ")));
                        }
                    }
                    vs.push(format!("AND ({})", vteg.join(" OR ")));
                }
                "color" => { 
                    let cgs = value.split("|"); 
                    let mut vcg = Vec::new();
                    for cg in cgs {
                        let mut vf = Vec::new();
                        let mut include = ">";
                        for c in cg.chars() {
                            let f = match c { //TODO: Speed test instr vs regex
                                'w' => { format!("instr(mana_cost, 'W') {} 0", include) }
                                'u' => { format!("instr(mana_cost, 'U') {} 0", include) }
                                'b' => { format!("instr(mana_cost, 'B') {} 0", include) }
                                'r' => { format!("instr(mana_cost, 'R') {} 0", include) }
                                'g' => { format!("instr(mana_cost, 'G') {} 0", include) }
                                'c' => { format!("mana_cost REGEXP \'^[^WUBRG]*$\'") }
                                '!' => { include = "="; continue;}
                                _ => { String::new() }
                            };
                            vf.push(f);
                        }
                        vcg.push(format!("({})", vf.join(" AND ")));
                    }
                    vs.push(format!("AND ({})", vcg.join(" OR ")));
                }
                "color_identity" => {
                    let cigs = value.split("|"); 
                    let mut vcig = Vec::new();
                    for cig in cigs {
                        let mut vf = Vec::new();
                        let mut include = ">";
                        for ci in cig.chars() {
                            let f = match ci { //TODO: Speed test instr vs regex
                                'w' => { format!("instr(color_identity, 'W') {} 0", include) }
                                'u' => { format!("instr(color_identity, 'U') {} 0", include) }
                                'b' => { format!("instr(color_identity, 'B') {} 0", include) }
                                'r' => { format!("instr(color_identity, 'R') {} 0", include) }
                                'g' => { format!("instr(color_identity, 'G') {} 0", include) }
                                'c' => { format!("color_identity REGEXP \'^[^WUBRG]*$\'") }
                                '!' => { include = "="; continue;}
                                _ => { String::new() }
                            };
                            vf.push(f);
                        }
                        vcig.push(format!("({})", vf.join(" AND ")));
                    }
                    vs.push(format!("AND ({})", vcig.join(" OR ")));
                }
                "type" => {
                    //TODO: Add expansion for supertypes and types
                    // rule types() = ['l' | 'e' | 'p' | 'i' | 's' | 'c' | 'a']

                    let tygs = value.split("|"); 
                    let mut vtyg = Vec::new();
                    for tyg in tygs {
                        let mut vf = Vec::new();
                        for mut ty in tyg.split('&') {
                            let include = match ty.get(0..1) {
                                Some("!") => { ty = ty.get(1..).unwrap(); "NOT LIKE" }
                                Some(_) => { "LIKE" }
                                None => { "" }
                            };
                            match ty {
                                "per" => { 
                                    // ty = "permanent"; 
                                    vf.push(format!("types NOT LIKE \'%instant%\'"));
                                    vf.push(format!("types NOT LIKE \'%sorcery%\'"));
                                    continue;
                                }
                                "l" => { ty = "legendary";}
                                "e" => { ty = "enchantment";}
                                "p" => { ty = "planeswalker";}
                                "i" => { ty = "instant";}
                                "s" => { ty = "sorcery";}
                                "c" => { ty = "creature";}
                                "a" => { ty = "artifact";}
                                "" => { continue; }
                                _ => {}
                            }
                            vf.push(format!("types {} \'%{}%\'", include, ty));
                        }
                        if vf.len() > 0 {
                            vtyg.push(format!("({})", vf.join(" AND ")));
                        }
                    }
                    vs.push(format!("AND ({})", vtyg.join(" OR ")));
                }
                "cmc" => {
                    match value.get(0..1) {
                        Some(">") => { vs.push(format!("AND cmc > {}", value.get(1..).unwrap())); }
                        Some("<") => { vs.push(format!("AND cmc < {}", value.get(1..).unwrap())); }
                        Some("-") => { vs.push(format!("AND cmc <= {}", value.get(1..).unwrap())); }
                        Some(_) => { 
                            match value.find("-") {
                                Some(i) => {
                                    let (min, max) = value.split_at(i);
                                    // let max = if let Some("") = max.get(1..) { i } else { "1000" };
                                    let max_raw = max.get(1..).unwrap();
                                    let max = if "" == max_raw { "1000" } else { max_raw };
                                    vs.push(format!("AND (cmc >= {} AND cmc <= {})", min, max));
                                }
                                None => {
                                    vs.push(format!("AND cmc = {}", value));
                                }
                            }
                        }
                        None => {}
                    }
                }
                "sort" => {
                    match value.get(0..1) {
                        Some("+") => { order = String::from("ASC"); }
                        Some("-") => { order = String::from("DESC"); }
                        Some(_) => {}
                        None => {}
                    }
                    sort_on = String::from(value.get(1..).unwrap());
                }
                //TODO: Add later. Not critical right now, and will be annoying due to the string->integer translation.
                // "strength" => {}
                // "toughness" => {}
                _ => {}
            }
        }
        vs.push(format!("ORDER BY {} {};", sort_on, order));

        vs.join("\n")
    }
}

pub fn add_regexp_function(db: &Connection) -> Result<()> {
    db.create_scalar_function(
        "regexp",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            assert_eq!(ctx.len(), 2, "called with unexpected number of arguments");
            let regexp: Arc<Regex> = ctx
                .get_or_create_aux(0, |vr| -> Result<_, BoxError> {
                    Ok(Regex::new(vr.as_str()?)?)
                })?;
            let is_match = {
                let text = ctx
                    .get_raw(1)
                    .as_str()
                    .map_err(|e| Error::UserFunctionError(e.into()))?;

                regexp.is_match(text)
            };

            Ok(is_match)
        },
    )
}

pub fn initdb(conn: &Connection) -> Result<()> {
    conn.execute(
        "create table if not exists rulings (
            id integer primary key,
            date text not null,
            text text not null 
        )", NO_PARAMS,)?;

    conn.execute(
        "create table if not exists sets (
            id integer primary key,
            code text not null unique, 
            name text not null unique
        )", NO_PARAMS,
    )?;

    conn.execute(
        "create table if not exists cards (
            id integer primary key,
            name text not null unique,
            mana_cost text not null,
            cmc integer not null,
            types text not null,
            card_text text,
            power text,
            toughness text,
            loyalty text,
            color_identity text,
            related_cards text,
            layout text not null,
            side text,
            legalities text not null,
            price real,
            date_price_retrieved text
        )", NO_PARAMS,
    )?;
    
    conn.execute(
        "create table if not exists decks (
            id integer primary key,
            name text not null,
            commander text not null,
            commander2 text,
            deck_type text not null,
            notes text,
            foreign key (commander) references cards(name),
            foreign key (commander2) references cards(name))"
            , NO_PARAMS,
        )?;

    conn.execute(
        "create table if not exists deck_contents2 (
            id integer primary key,
            card_name text not null,
            deck integer not null,
            tags text,
            foreign key (deck) references decks(id) ON DELETE CASCADE,
            unique (deck, card_name) on conflict ignore)"
            , NO_PARAMS,
    )?;

    Ok(())
}

pub fn ivcfjsmap(conn: &Connection, jm: Value) -> Result<(usize, usize)> {
    //TODO: since we've split the Json out of Card, see if any of this can be more effectively passed in
    let mut stmt = conn.prepare("INSERT INTO cards (
        name, mana_cost, cmc, types, card_text, power, toughness, loyalty, color_identity, related_cards, layout, side, legalities
    ) VALUES (
            :name, :mana_cost, :cmc, :types, :card_text, :power, :toughness, :loyalty, :color_identity, :related_cards, :layout, :side, :legalities
    )")?;
    let mut vc: Vec<JsonCard> = Vec::new();

    let (mut success, mut failure) = (0, 0);    
    
    let map =  match &jm["data"] {
        serde_json::Value::Object(i) => { i }
        _ => { panic!(); }
    };

    for (_name, value) in map {
        // For some reason, serde won't deserialize the sequence properly.
        for v in value.as_array() {
            for c in v {
                let d = serde_json::from_value(c.clone()).unwrap();
                vc.push(d);
            }
        }
    }

    println!("Generated card array.");
    conn.execute_batch("BEGIN TRANSACTION;")?;

    for c in vc {
        let mut name = c.name.clone();
        let mut side = String::new();
        let mut related = String::new();
        match c.layout.as_str() {
            "split" | "transform" | "aftermath" | 
            "flip" | "adventure" | "modal_dfc" => { 
                let names = c.name.split_once(" // ").unwrap();
                if Some('a') == c.side {
                    name = names.0.to_string();
                    related = names.1.to_string();
                    side = "a".to_string()
                } else {
                    name = names.1.to_string();
                    related = names.0.to_string();
                    side = "b".to_string()
                }
            }
            "meld" => {
                if Some('a') == c.side {
                    let names = c.name.split_once(" // ").unwrap();
                    name = names.0.to_string();
                    related = names.1.to_string();
                    side = "a".to_string()
                } else {
                    related = "UNKNOWN".to_string();
                    side = "b".to_string()
                }
            }
            _ => {}
        }

        let c = c.clone();

        match stmt.execute_named(named_params!{
            ":name": name,
            ":mana_cost": c.mana_cost,
            ":cmc": c.cmc,
            ":types": c.types,
            ":card_text": c.text,
            ":power": c.power,
            ":toughness": c.toughness,
            ":loyalty": c.loyalty,
            ":color_identity": c.color_identity.join("|"),
            ":related_cards": related,
            ":layout": c.layout,
            ":side": side,
            ":legalities": c.legalities.to_string(),
        }) {
            Ok(_) => { success += 1; },
            Err(_) => { 
                // Usually an unset
                failure += 1;
                println!("Failed for {}", name);
             },
        }
    }

    println!("Added all cards.");
    
    let _a = conn.execute("DELETE
        FROM cards
        WHERE legalities = \"\"", NO_PARAMS)?;
    
    println!("Deleted illegal cards.");
    
    conn.execute_batch("COMMIT TRANSACTION;")?;
    
    println!("Committed transaction.");

    // TODO: for each meld card with relation to unknown, automatically correct it.

    Ok((success, failure))
}

pub fn ictodc(conn: &Connection, c: &Card, did: i32) -> Result<Vec<Card>> {
    let mut r = Vec::new();
    let mut stmt = conn.prepare("INSERT INTO deck_contents (card_name, deck) VALUES (:card_name, :deck_id)")?;
    stmt.execute_named(named_params!{":card_name": c.name, ":deck_id": did as u32} )?;
    r.push(rcfn(conn, &c.name).unwrap());
    
    match &c.lo {
        crate::Layout::Flip(_, n) | 
        crate::Layout::Split(_, n) | 
        crate::Layout::ModalDfc(_, n) | 
        crate::Layout::Aftermath(_, n) | 
        crate::Layout::Adventure(_, n) | 
        crate::Layout::Transform(_, n) => { 
            stmt.execute_named(named_params!{":card_name": n, ":deck_id": did as u32} )?;
            r.push(rcfn(conn, &c.name).unwrap());
        }
        crate::Layout::Meld(s, n, m) => { 
            if s == &'b' {  
                stmt.execute_named(named_params!{":card_name": n, ":deck_id": did as u32} )?;
                r.push(rcfn(conn, &c.name).unwrap()); 
                stmt.execute_named(named_params!{":card_name": m, ":deck_id": did as u32} )?;
                r.push(rcfn(conn, &c.name).unwrap());
            } else {
                let names: Vec<String> =  rvcfdid(conn, did).unwrap().iter().map(|c| c.to_string()).collect();
                if names.contains(&n) {  
                    stmt.execute_named(named_params!{":card_name": m, ":deck_id": did as u32} )?;
                    r.push(rcfn(conn, &c.name).unwrap());
                }
            }
        }
        _ => {}
    }


    Ok(r)
}

pub fn dcntodc(conn: &Connection, c: &String, did: i32) -> Result<()> {
    let mut stmt = conn.prepare("DELETE FROM deck_contents WHERE card_name = :card_name AND deck = :deck_id")?;
    stmt.execute_named(named_params!{":card_name": c, ":deck_id": did as u32} )?;
    Ok(())
}

pub fn ttindc(conn: &Connection, c: String, t: &String, did: i32) -> Option<Card> {
    let mut card = rcfndid(conn, &c, did).unwrap();
    if card.tags.contains(&t) {
        card.tags.remove(card.tags.iter().position(|x| x == t).unwrap());
        // return None;
    } else {
        card.tags.push(t.clone());
    }
    conn.execute_named("UPDATE deck_contents 
        SET tags = :tags
        WHERE card_name = :name
        AND deck = :did;", named_params!{":tags": card.tags.join("|"), ":name": c, ":did": did})
        .unwrap();
    Some(card)
}

pub fn ideck(conn: &Connection, n: &String, c: &String, c2: Option<&String>, t: &str) -> Result<i32> {
    // if c2 == &String::new() { let c2 = None; }
    // let c2 = c2.unwrap_or(rusqlite::types::Null);
    match c2 {
        Some(c2) => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, commander2, deck_type) VALUES (:name, :commander, :commander2, :deck_type);").unwrap();
            stmt.execute_named(named_params!{":name": n, ":commander": c, ":commander2": c2, ":deck_type": t} ).unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, &c).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();
        
            let com = rcfn(conn, &c2).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();

            Ok(rid.try_into().unwrap())
            
        }
        None => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, deck_type) VALUES (:name, :commander, :deck_type);").unwrap();
            stmt.execute_named(named_params!{":name": n, ":commander": c, ":deck_type": t} ).unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, &c).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();

            Ok(rid.try_into().unwrap())
        }
    }
    // println!("Row ID is {}", rid);
}

pub fn import_deck(conn: &Connection, deck_name: String, com_name: String, cards: Vec<String>) -> Result<()> {
    let mut num = 0;
    if let Ok(_) = rcfn(conn, &com_name) {
        println!("Commander name is valid! Now creating deck...");
        //TODO: Add support for multi-commander decks
        //TODO: Check that the given string is an actual commander's name
        if let Ok(deck_id) = ideck(conn, &deck_name, &com_name, None, "Commander") {
            println!("Deck created successfully! Now adding cards...");
            conn.execute_batch("BEGIN TRANSACTION;")?;
            for c in cards {
                println!("Adding {}", c);
                let card = if let Some(i) = c.find(" // ") {
                    let c = c.get(0..i).unwrap();
                    rcfn(conn, &c.to_string()).unwrap()
                } else {
                    rcfn(conn, &c).unwrap()
                };
                ictodc(conn, &card, deck_id)?;
                num += 1;
            }
            conn.execute_batch("COMMIT TRANSACTION;")?;
        };
    };
    println!("Added {} cards to deck {}", num, deck_name);

    Ok(())
}

pub fn rcfn(conn: &Connection, name: &String) -> Result<Card> {
    let mut stmt = conn.prepare("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side
        FROM cards WHERE name = :name;")?;
    stmt.query_row_named(named_params!{":name": name}, |row| {
        cfr(row)
    })
}

pub fn rcfndid(conn: &Connection, name: &String, did: i32) -> Result<Card> {
    let mut stmt = conn.prepare("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags
        FROM cards 
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE cards.name = :name
        AND deck_contents.deck = :did;")?;
    stmt.query_row_named(named_params!{":name": name, ":did": did}, |row| {
        cfr(row)
    })
}

pub fn rcomfdid(conn: &Connection, did: i32, secondary: bool) -> Result<Card> {
    // let mut stmt = conn.prepare("SELECT commander FROM decks WHERE id = ?;")?;

    let name = if secondary { 
        conn.query_row("SELECT commander2 FROM decks WHERE id = ?;",
        params![did], 
        |row|{
            match row.get::<usize, _>(0) {
                Ok(i) => { Ok(i) }
                Err(a) => { return Err(a); }
            }
        })?
    } else {
        conn.query_row("SELECT commander FROM decks WHERE id = ?;",
        params![did], 
        |row|{
            Ok(row.get(0)?)
        })?
    };

    rcfn(conn, &name)
}

pub fn rvd (conn: &Connection) -> Result<Vec<Deck>> {
    let mut stmt = conn.prepare("SELECT * FROM decks;")?;

    let a = stmt.query_map(NO_PARAMS, |row| {
        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: rcfn(conn, &row.get(2)?)?,
        })
    })?;
    a.collect()

    // decks
}

pub fn rdfdid(conn: &Connection, id: i32) -> Result<Deck> {
    let mut stmt = conn.prepare("SELECT * FROM decks WHERE id = ?;")?;

    stmt.query_row(params![id], |row| {
        Ok( Deck {
            name: row.get(1)?,
            commander: rcfn(conn, &row.get(2)?)?,
            id: row.get(0)?,
        })
    })

    // let a = stmt.query_row(params, f)
}

pub fn rvcfdid(conn: &Connection, did: i32) -> Result<Vec<Card>> {
    let mut stmt = conn.prepare("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags
        FROM cards 
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did;")?;

    let a = stmt.query_map_named(named_params!{ ":did": did }, |row| { cfr(row) })?;
    a.collect()
}

pub fn rvcfcf(conn: &Connection, cf: CardFilter, general: bool) -> Result<Vec<Card>> {
    let fields = if general {
        "cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side"
    } else {
        "cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags"
    };
    let qs = format!("
        SELECT {}
        FROM `cards`
        {}", fields, cf.make_filter(conn, general));

    // println!("Preparing query:\n{}", qs);

    let mut stmt = conn.prepare(& qs).unwrap();

    let cards = stmt.query_map(NO_PARAMS, |row| {
        cfr(row)
    })?.collect();

    // println!("{:?}", cards);

    cards
}

pub fn rvcnfn(conn: &Connection, n: &String) -> Result<Vec<String>> {
    let query = format!("
        SELECT name
        FROM cards
        WHERE name LIKE \'%{}%\'
        AND types LIKE \'Legendary%\'
        AND (types LIKE \'%Creature%\' OR card_text LIKE \'%can be your commander%\')
        ORDER BY name ASC;", n);
    let mut stmt = conn.prepare(query.as_str())?;

    let a = stmt.query_map(
        NO_PARAMS, 
        |row| { 
            row.get(0)
        }
    )?;
    a.collect()
}

pub fn rvcnfnp(conn: &Connection, n: &String) -> Result<Vec<String>> {
    let query = format!("
        SELECT name
        FROM cards
        WHERE name LIKE '%{}%'
        AND types LIKE 'Legendary%'
        AND card_text LIKE '%Partner%'
        AND (types LIKE '%Creature%' OR card_text LIKE '%can be your commander%')
        ORDER BY name ASC;", n);
    let mut stmt = conn.prepare(query.as_str())?;

    let a = stmt.query_map(
        NO_PARAMS, 
        |row| { 
            row.get(0)
        }
    )?;
    a.collect()
}

fn stovs(ss: String) -> Vec<String> {
    let mut vs = Vec::new();

    for s in ss.split('|') {
        vs.push(String::from(s));
    }
    vs
}

fn cfr(row:& Row) -> Result<Card> {
    let lo = match row.get::<usize, String>(10) {
        Ok(s) => { 
            match s.as_str() {
                "adventure" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::Adventure(side, rel)    
                }
                "aftermath" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::Aftermath(side, rel)    
                }
                "flip" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::Flip(side, rel)    
                }
                "leveler" => { Layout::Leveler }
                "meld" => { 
                    let rel = row.get::<usize, String>(11)?;
                    let rels = rel.split_once("|").unwrap();
                    let (face, transform) = (String::from(rels.0), String::from(rels.1));
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::Meld(side, face, transform)
                }
                "modal_dfc" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::ModalDfc(side, rel)    
                }
                "normal" => { Layout::Normal }
                "saga" => { Layout::Saga }
                "split" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::Split(side, rel)    
                }
                "transform" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    Layout::Transform(side, rel)
                }
                _ => { Layout::Normal }
            }
         }
        Err(_) => { Layout::Normal }
    };
    
    let tags: Vec<String> = if row.column_names().contains(&"tags") { 
        // println!("In tags!");
        match row.get::<usize, String>(13) {
            Ok(a) => {
                stovs(a)
            }
            Err(_) => { Vec::new() }
        } 
    } else { Vec::new() };

    Ok( Card {
        cmc: row.get(0)?,
        color_identity: stovs(row.get(1)?),
        legalities: Legalities::from(row.get(2)?),
        loyalty: row.get(3)?,
        mana_cost: row.get(4)?,
        name: row.get(5)?,
        power: row.get(6)?,
        text: row.get(7)?,
        toughness: row.get(8)?,
        types: row.get(9)?,
        lo,
        tags
    })
}

pub fn rvmcfd(conn: &Connection, did: i32) -> Result<Vec<CardStat>> {
    let mut stmt = conn.prepare(r#"SELECT
        cmc, color_identity, mana_cost, name, tags, types, price
        FROM cards
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did
        AND (side != 'b' OR layout == 'split' OR layout == 'modal_dfc')
        AND tags IS NOT NULL 
        AND tags REGEXP '\|?main(?:$|\|)';"#).unwrap();
    let a = stmt.query_map_named(named_params!{":did": did}, |row| {
            Ok( CardStat {
                cmc: row.get::<usize, f64>(0)? as u8,
                color_identity: stovs(row.get(1)?),
                mana_cost: row.get(2)?,
                name: row.get(3)?,
                tags: stovs(row.get(4)?),
                types: row.get(5)?,
                price: if let Ok(i) = row.get(6) {
                    i
                } else {
                    0.0
                }
            })
        }
    )?.collect();
    a
}

pub fn ucfd(conn: &Connection, did: i32) -> Result<()> {
    let mut stmt = conn.prepare(r#"
        SELECT name, layout, related_cards, side, date_price_retrieved, tags
        FROM cards
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did
        AND side != 'b'
        AND (date_price_retrieved ISNULL OR date_price_retrieved < date('now','-6 day'))
        AND tags IS NOT NULL 
        AND tags REGEXP '\|?main(?:$|\|)';"#).unwrap();
    let delay = time::Duration::from_millis(100);
    let a: Result<Vec<(String, f64)>> = stmt.query_map_named(named_params!{":did": did}, |row| {
            thread::sleep(delay);
            Ok((row.get::<usize, String>(0)?, rpfdc(row)?))
        }
    )?.collect();

    stmt = conn.prepare("UPDATE cards 
    SET price = :price, 
    date_price_retrieved = date()
    WHERE name = :name;").unwrap();
    // a
    // let mut num = 0;
    for (name, price) in a.unwrap() {
        stmt.execute_named(named_params!{":price": price, ":name": name}).unwrap();
    }
    // println!("{} total prices updated.", num);
    Ok(())
}

pub fn rpfdc(row: &Row) -> Result<f64> {
    let layout: String = row.get(1)?;
    let related_cards: String = row.get(2)?;
    let s = if related_cards.len() > 0 
        && layout != String::from("meld") {
        format!("{} // {}", row.get::<usize, String>(0)?, row.get::<usize, String>(2)?)
    } else {
        row.get::<usize, String>(0)?
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let future = rcostfcn(&s);
    let res = rt.block_on(future).unwrap();

    // println!("{} has a price of {}", s, res);

    Ok(res)
}