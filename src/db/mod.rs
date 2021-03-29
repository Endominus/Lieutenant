extern crate rusqlite;
extern crate regex;

use crate::Legalities;
use self::rusqlite::{params, Connection, Result, NO_PARAMS, Error};
use crate::{Card, Deck, NewCard, Relation};
use std::{collections::HashMap, fs};
use rusqlite::{Row, Statement, named_params};
use serde::Deserialize;
use regex::Regex;
use serde_json::Value;
use self::rusqlite::functions::FunctionFlags;
use std::sync::Arc;
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

//TODO: Prepare statements and pass around a DB connection object as in https://tedspence.com/investigating-rust-with-sqlite-53d1f9a41112

// pub struct DbContext<'a> {
//     conn: Connection,
//     stmts: HashMap<&'a str, Statement<'a>>
// }

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

                rule fields(hm: &mut HashMap<&str, String>)
                = (text(hm) 
                / color(hm) 
                / ctype(hm) 
                / cmc(hm) 
                / color_identity(hm) 
                / power(hm) 
                / toughness(hm) 
                / name(hm))

                rule cmc(hm: &mut std::collections::HashMap<&str, String>)
                = "cmc:" value:$number_range() { hm.insert("cmc", String::from(value)); }
                rule name(hm: &mut HashMap<&str, String>)
                = name_alias()? value:ss_values() { hm.insert("name", value); }
                rule text(hm: &mut HashMap<&str, String>)
                = text_alias() ":" value:text_group() ** or_separator() { hm.insert("text", value.join("|")); }
                rule color(hm: &mut HashMap<&str, String>)
                = color_alias() ":" value:$(colors()+) ** or_separator() { hm.insert("color", value.join("|")); }
                rule ctype(hm: &mut HashMap<&str, String>)
                = type_alias() ":" value:type_group() ** or_separator() { hm.insert("type", value.join("|")); }
                rule power(hm: &mut std::collections::HashMap<&str, String>)
                = power_alias() ":" value:$((['0'..='9']+)? "-"? (['0'..='9']+)?) { hm.insert("power", String::from(value)); }
                rule toughness(hm: &mut std::collections::HashMap<&str, String>)
                = toughness_alias() ":" value:$((['0'..='9']+)? "-"? (['0'..='9']+)?) { hm.insert("toughness", String::from(value)); }
                rule color_identity(hm: &mut HashMap<&str, String>)
                = color_identity_alias() ":" value:$(colors()+) ** or_separator() { hm.insert("color_identity", value.join("|")); }

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

                rule word() -> String
                = s:$("!"? ['a'..='z' | 'A'..='Z' | '0'..='9' | '{' | '}' | '/']+) { String::from(s) }
                rule phrase() -> String
                =s:$("!"? "\"" (word() / " " / "+")+ "\"" ) {String::from(s) }
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
        
        hm
    }

    pub fn make_filter(&self, conn: &Connection, general: bool) -> String {
        let initial = match general {
            true => { 
                let com = rcomfdid(conn, self.did).unwrap();
                let mut colors = String::from("WUBRG");
                for ci in com.color_identity {
                    colors = colors.replace(&ci, "");
                }
                format!("WHERE color_identity REGEXP \'^[^{}]*$\'", colors) }
            false => { format!("
                INNER JOIN deck_contents
                ON cards.name = deck_contents.card_name
                WHERE deck_contents.deck = {}", self.did) }
        };
        
        let mut vs = Vec::from([initial]);

        for (key, value) in self.fi.clone() {
            match key {
                "name" => { vs.push(format!("AND (cards.name LIKE \'%{}%\')", value)); }
                "text" => { 
                    let tegs = value.split("|"); 
                    let mut vteg = Vec::new();
                    for teg in tegs {
                        let mut vf = Vec::new();
                        for mut te in teg.split('&') {
                            let include = match te.get(0..1) {
                                Some("!") => { te = te.get(1..).unwrap(); "NOT LIKE" }
                                Some(_) => { "LIKE" }
                                None => { "" }
                            };
                            vf.push(format!("card_text {} \'%{}%\'", include, te.trim_matches('\"')));
                        }
                        vteg.push(format!("({})", vf.join(" AND ")));
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
                                'b' => { format!("instr(color_identity, 'R') {} 0", include) }
                                'r' => { format!("instr(color_identity, 'B') {} 0", include) }
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
                            vf.push(format!("types {} \'%{}%\'", include, ty));
                        }
                        vtyg.push(format!("({})", vf.join(" AND ")));
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
                                    vs.push(format!("AND (cmc >= {} AND cmc <= {}", min, max.get(1..).unwrap()));
                                }
                                None => {}
                            }
                        }
                        None => {}
                    }
                }
                //TODO: Add later. Not critical right now, and will be annoying due to the string->integer translation.
                // "strength" => {}
                // "toughness" => {}
                _ => {}
            }
        }

        vs.join("\n")
    }
}

//TODO: Write public function to retrieve all cards. Remove layouts scheme, planar, and vanguard

// impl<'a> DbContext<'a> {
//     pub fn new(dbfile: &str) -> Result<DbContext> {
//         let conn = Connection::open(dbfile)?;
//         DbContext::add_regexp_function(&conn);
//         let stmts = HashMap::new();

        // stmts.insert("ic", 
        // conn.prepare("INSERT INTO cards (
        //     name, mana_cost, cmc, types, card_text, power, toughness, color_identity, related_cards, layout, side, legalities
        //     VALUES (
        //         :name, :mana_cost, :cmc, :types, :card_text, :power, :toughness, :color_identity, :related_cards, :layout, :side, :legalities
        //     )")?);
        // stmts.insert("iset", conn.prepare("INSERT INTO sets (code, name) VALUES (:code, :name);")?);
        // stmts.insert("icntodc", conn.prepare("INSERT INTO deck_contents (card_name, deck) VALUES (:card_name, :deck_id)")?);
        // stmts.insert("ideck", conn.prepare("INSERT INTO decks (name, commander, deck_type) VALUES (:name, :commander, :deck_type);")?);
        // stmts.insert("rcfn", 
        // conn.prepare("SELECT cmc, color_identity, legalities, mana_cost, name, power, text, toughness, types, layout, related_cards, side
        //     FROM cards WHERE name = :name;")?);
        // stmts.insert("rvcfdid", 
        // conn.prepare("SELECT 
        //     cmc, color_identity, legalities, mana_cost, name, power, text, toughness, types, layout, related_cards, side
        //     FROM cards 
        //     INNER JOIN deck_contents
        //     ON cards.name = deck_contents.card_name
        //     WHERE deck_contents.deck = :did;")?);
        // stmts.insert("rvd", conn.prepare("SELECT * FROM decks;")?);
        // stmts.insert("rdfdid", conn.prepare("SELECT * FROM decks WHERE id = ?;")?);
            
    //     Ok(DbContext { conn, stmts })
    // }

// }

fn add_regexp_function(db: &Connection) -> Result<()> {
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
            legalities text not null
        )", NO_PARAMS,
    )?;
    
    conn.execute(
        "create table if not exists decks (
            id integer primary key,
            name text not null,
            commander text not null,
            deck_type text not null,
            notes text,
            cost real,
            date_cost_retrieved text,
            foreign key (commander) references cards(name))"
            , NO_PARAMS,)?;

    conn.execute(
        "create table if not exists deck_contents (
            id integer primary key,
            card_name text not null,
            deck integer not null,
            tags text,
            foreign key (deck) references decks(id))"
            , NO_PARAMS,
    )?;

    Ok(())
}

pub fn ivcfjsmap(conn: &Connection, jm: Value) -> Result<(usize, usize)> {
    let mut stmt = conn.prepare("INSERT INTO cards (
        name, mana_cost, cmc, types, card_text, power, toughness, loyalty, color_identity, related_cards, layout, side, legalities
    ) VALUES (
            :name, :mana_cost, :cmc, :types, :card_text, :power, :toughness, :loyalty, :color_identity, :related_cards, :layout, :side, :legalities
    )")?;
    let mut vc: Vec<NewCard> = Vec::new();

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
        // let (name, side, related) = match c.layout.as_str() {
        //     "split" | "transform" | "aftermath" | 
        //     "flip" | "adventure" | "modal_dfc" => { 
        //         let test = c.name.clone();
        //         let (a, b) = test.split_once(" // ").unwrap();
        //         if Some('a') == c.side { (a, "a", b) } 
        //         else { (b, "b", a) }
        //     }
        //     "meld" => { 
        //         if Some('a') == c.side {
        //             let test = c.name.clone();
        //             let (a, b) = test.split_once(" // ").unwrap();
        //             (a, "a", b)
        //         } else {
        //             (c.name.as_str(), "b", "unknown")
        //         }
        //      }
        //     _ => { (c.name.as_str(), "", "") }
        // };

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

// // TODO: is, icntodc, and ideck can all be collapsed into one function.
// pub fn iset (conn: Connection, s: Set) -> Result<()> {
//     let mut stmt = stmts.get("iset").unwrap();
//     stmt.execute_named(named_params!{":code": s.code, ":name": s.name} )?;
//     Ok(())
// }
pub fn icntodc(conn: &Connection, c: String, did: usize) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO deck_contents (card_name, deck) VALUES (:card_name, :deck_id)")?;
    stmt.execute_named(named_params!{":card_name": c, ":deck": did as u32} )?;
    Ok(())
}
pub fn ideck(conn: &Connection, n: String, c: Card, t: String) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO decks (name, commander, deck_type) VALUES (:name, :commander, :deck_type);")?;
    stmt.execute_named(named_params!{":name": n, ":commander": c.name,":deck_type": t} )?;
    Ok(())
}

pub fn import_deck(conn: &Connection, vc: Vec<String>, deck_id: usize) -> Result<()> {
    conn.execute_batch("BEGIN TRANSACTION;")?;
    for c in vc {
        icntodc(conn, c, deck_id)?;
    }
    conn.execute_batch("COMMIT TRANSACTION;")?;

    Ok(())
}

pub fn rcfn(conn: &Connection, name: String) -> Result<NewCard> {
    let mut stmt = conn.prepare("SELECT cmc, color_identity, legalities, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side
        FROM cards WHERE name = :name;")?;
    stmt.query_row_named(named_params!{":name": name}, |row| {
        cfr(row)
    })
}

pub fn rcomfdid(conn: &Connection, did: i32) -> Result<NewCard> {
    // let mut stmt = conn.prepare("SELECT commander FROM decks WHERE id = ?;")?;

    let name = conn.query_row("SELECT commander FROM decks WHERE id = ?;",
    params![did], 
    |row|{
        Ok(row.get(0)?)
    })?;// .query_row(params![did], |row| {

    rcfn(conn, name)
}

pub fn rvd (conn: &Connection) -> Result<Vec<Deck>> {
    let mut stmt = conn.prepare("SELECT * FROM decks;")?;

    let a = stmt.query_map(NO_PARAMS, |row| {
        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: rcfn(conn, row.get(2)?)?,
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
            commander: rcfn(conn, row.get(2)?)?,
            id: row.get(0)?,
        })
    })

    // let a = stmt.query_row(params, f)
}

pub fn rvcfdid(conn: &Connection, did: i32) -> Result<Vec<NewCard>> {
    let mut stmt = conn.prepare("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, text, toughness, types, layout, related_cards, side
        FROM cards 
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did;")?;

    let a = stmt.query_map(NO_PARAMS, |row| { cfr(row)})?;
    a.collect()
}

pub fn rvcfcf(conn: &Connection, cf: CardFilter, general: bool) -> Result<Vec<NewCard>> {
    let qs = format!("
        SELECT 
            name, 
            card_text, 
            mana_cost,
            layout, 
            types, 
            color_identity, 
            related_cards, 
            power, 
            toughness, 
            cmc
        FROM `cards`
        {}
        ORDER BY name;", cf.make_filter(conn, general));

    let mut stmt = conn.prepare(& qs)?;

    let cards = stmt.query_map(NO_PARAMS, |row| {
        cfr(row)
    })?.collect();

    cards
}


fn stovs(ss: String) -> Vec<String> {
    let mut vs = Vec::new();

    for s in ss.split('|') {
        vs.push(String::from(s));
    }
    vs
}

fn cfr(row:& Row) -> Result<NewCard> {
    let n = "normal".to_string();
    let m = "meld".to_string();
    let l = "leveler".to_string();
    let s = "saga".to_string();
    let (side, related_cards) = match row.get::<usize, String>(10) {
        Ok(n) => { (None, None) }
        Ok(l) => { (None, None) }
        Ok(s) => { (None, None) }
        Ok(m) => { 
            let a = row.get::<usize, String>(11)?;
            let b = a.split_once("|").unwrap();
            let (face, transform) = (String::from(b.0), String::from(b.1));
            (
                Some(row.get::<usize, String>(12)?.chars().next().unwrap()), 
                Some(Relation::Meld{ face: String::from(face), transform: String::from(transform) }) 
            )
        }
        Ok(_) => { (
            Some(row.get::<usize, String>(12)?.chars().next().unwrap()), 
            Some(Relation::Single(row.get(11)?))) }
        Err(_) => { (None, None) }
    };
    let tags: Vec<String> = if row.column_names().contains(&"tags") 
        { row.get::<usize, String>(13)?.split("|").map(|s| s.to_string()).collect() }
        else { Vec::new() };

    Ok( NewCard {
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
        layout: row.get(10)?,
        related_cards,
        side,
        tags
    })
}
