extern crate rusqlite;
extern crate regex;
// extern crate tokio;

use crate::util::{Layout, CardStat, Card, Deck, CommanderType};

use self::rusqlite::{params, Connection};
use std::{collections::HashMap, convert::TryInto, sync::Mutex};
use rusqlite::{Row, named_params, Result, Error};
use serde::{Deserialize, Serialize};
use regex::Regex;
use serde_json::Value;
use self::rusqlite::functions::FunctionFlags;
use std::sync::Arc;
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
use std::{thread, time};

// use anyhow::{};

// use futures::{io::BufReader, prelude::*};
// use tokio::prelude::*;

// pub struct DbContext<'a> {
//     conn: Connection,
//     stmts: HashMap<&'a str, Statement<'a>>
// }

use crate::network::rcostfcn;
use crate::util::{SortOrder, DefaultFilter};

const DB_FILE: &str = "lieutenant.db";

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Set {
    pub code: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportCard {
    pub name: String,
    pub tags: Option<String>,
}

impl PartialEq for ImportCard {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Default)]
pub struct CardFilter<'a> {
    did: i32,
    color: String,
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

    pub fn from(deck: &Deck, omni: &'a String, default_filter: DefaultFilter) -> CardFilter<'a> {
        let mut cf = CardFilter::default();

        cf.did = deck.id;
        cf.color = deck.color.clone();
        if omni.len() > 0 { 
            cf.fi = CardFilter::parse_omni(omni.as_str(), default_filter); 
        } else {
            cf.fi = HashMap::new();
        }

        cf
    }

    pub fn parse_omni(omni: &str, default_filter: DefaultFilter) -> HashMap<&str, String> {
        let mut hm = HashMap::new();
        // hm.insert(String::from("test"), String::from("test"));

        // let omni = String::from(omni);

        peg::parser!{
            grammar omni_parser() for str {
                pub rule root(hm: &mut HashMap<&str, String>)
                = (fields(hm) / " "+)+ ![_]

                pub rule default() -> String
                =s:$((word() / " " / "+" / ":" / "/")+) ** " " { s.join(" ") }

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
                = name_alias() ":" value:ss_values() { hm.insert("name", value); }
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

                rule name_alias() = ("name" / "n")
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

        // match omni_parser::root(omni, &mut hm) {
        //     Ok(_) => {}
        //     Err(_) => {}
        // }
        let _a = omni_parser::root(omni, &mut hm);
        if hm.is_empty() {
            let mut ss = omni;
            if let Some(i) = omni.find(" /") {
                ss = omni.get(0..i).unwrap();
            }
            let ph = omni_parser::default(ss.trim()).unwrap();
            match default_filter {
                DefaultFilter::Name => { hm.insert("name", ph); }
                DefaultFilter::Text => { hm.insert("text", ph); }
                // DefaultFilter::Name => { hm.insert("name", String::from(omni)); }
                // DefaultFilter::Text => { hm.insert("text", String::from(omni)); }
            }
        }
        
        hm
    }

    pub fn make_filter(&self, general: bool, sort_order: SortOrder) -> String {
        let initial = match general {
            true => { 
                // let com = rcomfdid(conn, self.did, false).unwrap();
                let mut colors = String::from("WUBRG");
                // for ci in com.color_identity {
                //     colors = colors.replace(&ci, "");
                // }

                // if let Ok(com) = rcomfdid(conn, self.did, true) {
                //     for ci in com.color_identity {
                //         colors = colors.replace(&ci, "");
                //     }
                // }
                for c in self.color.chars() {
                    colors = colors.replace(c, "");
                }
                format!("
                    LEFT OUTER JOIN deck_contents
                    ON cards.name = deck_contents.card_name
                    WHERE color_identity REGEXP \'^[^{}]*$\'", colors) 
            }
            false => { 
                format!("
                    INNER JOIN deck_contents
                    ON cards.name = deck_contents.card_name
                    WHERE deck_contents.deck = {}", self.did) 
            }
        };

        let (mut order, mut sort_on) = match sort_order {
            SortOrder::NameAsc => { (String::from("ASC"), String::from("name")) }
            SortOrder::NameDesc => { (String::from("DESC"), String::from("name")) }
            SortOrder::CmcAsc => { (String::from("ASC"), String::from("cmc")) }
            SortOrder::CmcDesc => { (String::from("DESC"), String::from("cmc")) }
        };

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
                // "rarity" => {}
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
        )", [],)?;

    conn.execute(
        "create table if not exists sets (
            id integer primary key,
            code text not null unique, 
            name text not null unique
        )", [],
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
        )", [],
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
            , [],
        )?;

    conn.execute(
        "create table if not exists deck_contents2 (
            id integer primary key,
            card_name text not null,
            deck integer not null,
            tags text,
            foreign key (deck) references decks(id) ON DELETE CASCADE,
            unique (deck, card_name) on conflict ignore)", [])?;

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

        match stmt.execute(named_params!{
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
        WHERE legalities = \"\"", [])?;
    
    println!("Deleted illegal cards.");
    
    conn.execute_batch("COMMIT TRANSACTION;")?;
    
    println!("Committed transaction.");

    // TODO: for each meld card with relation to unknown, automatically correct it.

    Ok((success, failure))
}

pub fn ictodc(conn: &Connection, c: &Card, did: i32) -> Result<Vec<Card>> {
    let mut r = Vec::new();
    let mut stmt = conn.prepare("INSERT INTO deck_contents (card_name, deck) VALUES (:card_name, :deck_id)")?;
    stmt.execute(named_params!{":card_name": c.name, ":deck_id": did as u32} )?;
    r.push(rcfn(conn, &c.name, None).unwrap());
    
    match &c.lo {
        Layout::Flip(_, n) | 
        Layout::Split(_, n) | 
        Layout::ModalDfc(_, n) | 
        Layout::Aftermath(_, n) | 
        Layout::Adventure(_, n) | 
        Layout::Transform(_, n) => { 
            stmt.execute(named_params!{":card_name": n, ":deck_id": did as u32} )?;
            r.push(rcfn(conn, &c.name, None).unwrap());
        }
        Layout::Meld(s, n, m) => { 
            if s == &'b' {  
                stmt.execute(named_params!{":card_name": n, ":deck_id": did as u32} )?;
                r.push(rcfn(conn, &c.name, None).unwrap()); 
                stmt.execute(named_params!{":card_name": m, ":deck_id": did as u32} )?;
                r.push(rcfn(conn, &c.name, None).unwrap());
            } else {
                let names: Vec<String> =  rvcfdid(conn, did, SortOrder::NameAsc).unwrap().iter().map(|c| c.to_string()).collect();
                if names.contains(&n) {  
                    stmt.execute(named_params!{":card_name": m, ":deck_id": did as u32} )?;
                    r.push(rcfn(conn, &c.name, None).unwrap());
                }
            }
        }
        _ => {}
    }


    Ok(r)
}

pub fn dcntodc(conn: &Connection, c: &String, did: i32) -> Result<()> {
    let mut stmt = conn.prepare("DELETE FROM deck_contents WHERE card_name = :card_name AND deck = :deck_id")?;
    stmt.execute(named_params!{":card_name": c, ":deck_id": did as u32} )?;
    Ok(())
}

pub fn ttindc(conn: &Connection, c: &String, t: &String, did: i32) -> Option<Card> {
    let mut card = rcfndid(conn, c, did).unwrap();
    let tags = if card.tags.contains(&t) {
        card.tags.remove(card.tags.iter().position(|x| x == t).unwrap());
        // return None;
        if card.tags.is_empty() {
            None
        } else {
            Some(card.tags.join("|"))
        }
    } else {
        card.tags.push(t.clone());
        Some(card.tags.join("|"))
    };
    conn.execute("UPDATE deck_contents 
        SET tags = :tags
        WHERE card_name = :name
        AND deck = :did;", named_params!{":tags": tags, ":name": c, ":did": did})
        .unwrap();
    Some(card)
}

pub fn cindid(conn: &Connection, c: &String, did: i32) -> bool {
    let a = conn.query_row("SELECT card_name FROM deck_contents WHERE card_name = ? AND deck = ?;", 
    params![c, did], |_| Ok(0));

    match a {
        Ok(_) => { true }
        Err(_) => { false }
    }
}

pub fn ideck(conn: &Connection, n: &String, c: &String, c2: Option<String>, t: &str) -> Result<i32> {
    // if c2 == &String::new() { let c2 = None; }
    // let c2 = c2.unwrap_or(rusqlite::types::Null);
    match c2 {
        Some(c2) => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, commander2, deck_type) VALUES (:name, :commander, :commander2, :deck_type);").unwrap();
            stmt.execute(named_params!{":name": n, ":commander": c, ":commander2": c2, ":deck_type": t} ).unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, &c, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();
        
            let com = rcfn(conn, &c2, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();

            Ok(rid.try_into().unwrap())
            
        }
        None => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, deck_type) VALUES (:name, :commander, :deck_type);").unwrap();
            stmt.execute(named_params!{":name": n, ":commander": c, ":deck_type": t} ).unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, &c, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();

            Ok(rid.try_into().unwrap())
        }
    }
    // println!("Row ID is {}", rid);
}

pub fn import_deck(conn: &Connection, deck_name: String, coms: Vec<String>, cards: Vec<ImportCard>) -> Result<()> {
    let mut num = 0;
    let (primary, secondary) = match coms.len() {
        0 => { 
            let c = rcfn(conn, &cards.first().unwrap().name, None).unwrap();
            match c.is_commander() {
                CommanderType::Default => { 
                    println!("Valid commander found: {}", &c.name);
                    (&cards.first().unwrap().name, None) 
                }
                CommanderType::Partner => { 
                    if let Some(cn) = cards.get(1) {
                        let sc = rcfn(conn, &cn.name, None).unwrap();
                        if sc.is_commander() == CommanderType::Partner { 
                            println!("Valid commanders found: {} and {}", c.name, sc.name);
                            // This is gross, but we need the owned value
                            (&cards.first().unwrap().name, Some(cards.get(1).unwrap().name.clone()))
                        } else { 
                            println!("Valid commander found: {}", c.name);
                            println!("This commander has the Partner keyword. To include the partner as a secondary commander, it must be the second card in the file.");
                            (&cards.first().unwrap().name, None) 
                        }
                    } else { 
                        println!("Valid commander found: {}", c.name);
                        println!("Did you really import a deck of just one card? Why?");
                        (&cards.first().unwrap().name, None) 
                    }
                }
                CommanderType::PartnerWith(ss) => { 
                    let partner = ImportCard { name: ss.clone(), tags: None};
                    if cards.contains(&partner) {
                        println!("Valid commanders found: {} and {}", c.name, &ss);
                        // let ss = ss.clone();
                        (&cards.first().unwrap().name, Some(ss))
                    } else {
                        println!("Valid commander found: {}", c.name);
                        println!("But did you forget to put {} in the deck? It wasn't found.", &ss);
                        (&cards.first().unwrap().name, None) 
                    }
                }
                CommanderType::Invalid  => { 
                    // (coms.first().unwrap(), None)
                    println!("No valid commander found! Please ensure you pass in your commander name or include it at the top of the import file.");
                    return Ok(())
                }
            }
        }
        1 => { 
            println!("Valid commander found: {}", coms.first().unwrap());
            (coms.first().unwrap(), None) 
        }
        _ => {
            (coms.first().unwrap(), None)
        }
    };
    if let Ok(deck_id) = ideck(conn, &deck_name, &primary, secondary, "Commander") {
        println!("Deck created successfully! Now adding cards...");
        conn.execute_batch("BEGIN TRANSACTION;")?;
        let deck = rdfdid(conn, deck_id).unwrap();
        for ic in cards {
            // println!("Adding {}", c);
            let c = ic.name.trim().to_string();
            if c.len() == 0 { continue }
            let card = if let Some(i) = c.find(" // ") {
                let c = c.get(0..i).unwrap();
                rcfn(conn, &c.to_string(), None).unwrap()
            } else {
                // rcfn(conn, &c).unwrap()
                match rcfn(conn, &c, None) {
                    Ok(a) => { a }
                    Err(_) => { println!("Error on card {}", c); return Ok(()) }
                }
            };
            let mut disq = "";
            for c in &card.color_identity {
                if *c != '\u{0}' && !deck.color.contains(*c) {
                    disq = "Invalid color identity";
                }
            }
            if disq.len() == 0 {
                ictodc(conn, &card, deck_id)?;
                if let Some(tags) = ic.tags {
                    for tag in tags.split("|") {
                        ttindc(conn, &card.name, &tag.to_string(), deck_id);
                    }
                };
                num += 1;
            } else {
                println!("Card not added: \"{}\" due to: {}", &card.name, disq);
            }
        }
        conn.execute_batch("COMMIT TRANSACTION;")?;
    };
    println!("Added {} cards to deck {}", num, deck_name);

    Ok(())
}

pub fn rcfn(conn: &Connection, name: &String, odid: Option<i32>) -> Result<Card> {
    let mut stmt = conn.prepare("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags
        FROM cards 
        LEFT OUTER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        AND deck_contents.deck = :did
        WHERE name = :name;")?;
    stmt.query_row(named_params!{":name": name, ":did": odid}, |row| {
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
    stmt.query_row(named_params!{":name": name, ":did": did}, |row| {
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

    rcfn(conn, &name, Some(did))
}

pub fn rvd (conn: &Connection) -> Result<Vec<Deck>> {
    let mut stmt = conn.prepare("SELECT * FROM decks;")?;

    let a = stmt.query_map([], |row| {
        let mut color = String::new();
        let com = rcfn(conn, &row.get(2)?, None)?;
        let mut com2_colors = Vec::new();
        let com2 = match row.get::<usize, String>(3) {
            Ok(com) => { 
                let b = rcfn(conn, &com, None)?; 
                com2_colors = b.color_identity.clone(); 
                Some(b) 
            }
            Err(_) => { None }
        };

        for c in "WUBRG".chars() {
            if com.color_identity.contains(&c)
                | com2_colors.contains(&c) {
                color.push(c);
            }
        }

        if color.len() == 0 {
            color = String::from("C");
        }

        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: com,
            commander2: com2,
            color
        })
    })?;
    a.collect()

    // decks
}

pub fn rdfdid(conn: &Connection, id: i32) -> Result<Deck> {
    let mut stmt = conn.prepare("SELECT * FROM decks WHERE id = ?;")?;

    stmt.query_row(params![id], |row| {
        let mut color = String::new();
        let com = rcfn(conn, &row.get(2)?, None)?;
        let mut com2_colors = Vec::new();
        let com2 = match row.get::<usize, String>(3) {
            Ok(com) => { 
                let b = rcfn(conn, &com, None)?; 
                com2_colors = b.color_identity.clone(); 
                Some(b) 
            }
            Err(_) => { None }
        };

        for c in "WUBRG".chars() {
            if com.color_identity.contains(&c)
                | com2_colors.contains(&c) {
                color.push(c);
            }
        }

        Ok( Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: com,
            commander2: com2,
            color
        })
    })

    // let a = stmt.query_row(params, f)
}

pub fn rvcfdid(conn: &Connection, did: i32, sort_order: SortOrder) -> Result<Vec<Card>> {
    let (order, sort_on) = match sort_order {
        SortOrder::NameAsc => { (String::from("ASC"), String::from("name")) }
        SortOrder::NameDesc => { (String::from("DESC"), String::from("name")) }
        SortOrder::CmcAsc => { (String::from("ASC"), String::from("cmc")) }
        SortOrder::CmcDesc => { (String::from("DESC"), String::from("cmc")) }
    };

    // For some reason, sqlite doesn't like named parameters in the ORDER BY clause.
    let s = format!("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags
        FROM cards 
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did
        ORDER BY {} {};", sort_on, order);
    

    let mut stmt = conn.prepare(s.as_str())?;
    
    let a = stmt.query_map(named_params!{ ":did": did }, |row| { cfr(row) })?;
    a.collect()
}

pub fn rvcfcf(conn: &Connection, cf: CardFilter, general: bool, sort_order: SortOrder) -> Result<Vec<Card>> {
    let fields = "cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags";
    let qs = format!("
        SELECT {}
        FROM `cards`
        {}", fields, cf.make_filter(general, sort_order));

    let mut stmt = conn.prepare(& qs).unwrap();

    let cards = stmt.query_map([], |row| {
        cfr(row)
    })?.collect();

    // println!("{:?}", cards);

    cards
}

pub fn rvcnfn(conn: &Connection, n: &String) -> Result<Vec<String>> {
    if n.len() == 0 {
        return Ok(Vec::new())
    }
    let query = format!("
        SELECT name
        FROM cards
        WHERE name LIKE \'%{}%\'
        AND types LIKE \'Legendary%\'
        AND (types LIKE \'%Creature%\' OR card_text LIKE \'%can be your commander%\')
        ORDER BY name ASC;", n);
    let mut stmt = conn.prepare(query.as_str())?;

    let a = stmt.query_map([], 
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

    let a = stmt.query_map([], 
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

fn stovch(sch: String) -> Vec<char> {
    let mut vch = Vec::new();

    for ch in sch.split("|") {
        vch.push(ch.chars().next().unwrap_or_default());
        // println!("{:?}", vch);
    }
    vch
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
        color_identity: stovch(row.get(1)?),
        // legalities: Legalities::from(row.get(2)?),
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

pub fn rvicfdid(conn: &Connection, did: i32) -> Result<Vec<ImportCard>> {
    // let mut r = Vec::new();
    let mut stmt = conn.prepare(r#"SELECT
        card_name, tags
        FROM deck_contents
        WHERE deck = :did;"#).unwrap();

    let r = stmt.query_map(named_params!{":did": did}, |row| {
        Ok(ImportCard { name: row.get(0)?, tags: row.get(1)? })
    })?.collect();

    r
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
    let a = stmt.query_map(named_params!{":did": did}, |row| {
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

pub fn ucfd(rwl_conn: &Mutex<Connection>, did: i32) -> Result<()> {
        let unpriced: Result<Vec<(String, String, String)>> = {
            let conn = rwl_conn.lock().unwrap();
            let mut stmt = conn.prepare(r#"
                SELECT name, layout, related_cards
                FROM cards
                INNER JOIN deck_contents
                ON cards.name = deck_contents.card_name
                WHERE deck_contents.deck = :did
                AND side != 'b'
                AND (date_price_retrieved ISNULL OR date_price_retrieved < date('now','-6 day'))
                AND tags IS NOT NULL 
                AND tags REGEXP '\|?main(?:$|\|)';"#).unwrap();
            let a = stmt.query_map(
                named_params!{":did": did}, 
                |row| {
                    Ok((row.get::<usize, String>(0)?, row.get::<usize, String>(1)?, row.get::<usize, String>(2)?))
            })?.collect();

            a
        };
            
        let delay = time::Duration::from_millis(50);
        for (name, layout, related) in unpriced.unwrap() {
            thread::sleep(delay);
            let price = rpfdc(&name, &layout, &related).unwrap();
            rwl_conn.lock().unwrap().execute("UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;", 
                named_params!{":price": price, ":name": name})?;
        }
    
    Ok(())
}

pub fn dd(conn: &Connection, did: i32) -> Result<()> {
    conn.execute("DELETE FROM decks WHERE id = :did", named_params!{":did": did})?;
    Ok(())
}

pub fn rpfdc(name: &String, layout: &String, related: &String) -> Result<f64> {
    let s = if related.len() > 0 
        && layout != &String::from("meld") {
        format!("{} // {}", name, related)
    } else {
        name.clone()
    };

    // let rt = tokio::runtime::Runtime::new().unwrap();
    // let future = rcostfcn(&s);
    // let res = rt.block_on(future).unwrap();
    let res = rcostfcn(&s).unwrap();
    // println!("{} has a price of {}", s, res);

    Ok(res)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonCard {
    #[serde(rename = "convertedManaCost")]
    pub cmc: f64,
    pub color_identity: Vec<String>,
    pub legalities: Legalities,
    #[serde(default)]
    pub loyalty: String,
    #[serde(default = "zero")]
    pub mana_cost: String,
    pub name: String,
    #[serde(default)]
    pub power: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub toughness: String,
    #[serde(rename = "type")]
    pub types: String,
    pub layout : String,
    // pub related_cards: Option<Relation>,
    pub side: Option<char>,
    //TODO: Add rarity and sets
}

fn zero() -> String { String::from("0") }

impl JsonCard {
    pub fn convert(&self) -> Card { todo!(); }
}



#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Legalities {
    #[serde(default)]
    brawl: String,
    #[serde(default)]
    commander: String,
    #[serde(default)]
    duel: String,
    #[serde(default)]
    future: String,
    #[serde(default)]
    frontier: String,
    #[serde(default)]
    historic: String,
    #[serde(default)]
    legacy: String,
    #[serde(default)]
    modern: String,
    #[serde(default)]
    pauper: String,
    #[serde(default)]
    penny: String,
    #[serde(default)]
    pioneer: String,
    #[serde(default)]
    standard: String,
    #[serde(default)]
    vintage: String,
}

impl Legalities {
    fn to_vec(map: serde_json::Value) -> Vec<String> {
        let legalities = match map {
            serde_json::Value::Object(i) => { i }
            _ => { return Vec::new() }
        };

        legalities.keys().cloned().collect()
    }

    fn from(s: String) -> Legalities {
        let mut l = Legalities::default();

        if let Some(_) = s.find("commander") { l.commander = String::from("Allowed"); }

        l
    }
}

impl ToString for Legalities { 
    fn to_string(& self) -> String {
        let mut vs = Vec::new();
        let b = vec![String::default(), String::from("Banned")];

        if !b.contains(&self.brawl) { vs.push("brawl"); }
        if !b.contains(&self.commander) { vs.push("commander"); }
        if !b.contains(&self.modern) { vs.push("modern"); }
        if !b.contains(&self.standard) { vs.push("standard"); }

        vs.join("|")
    }
}