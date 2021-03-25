extern crate rusqlite;
extern crate regex;

use self::rusqlite::{params, Connection, Result, NO_PARAMS, Error};
use crate::{Card, Deck};
use std::{collections::HashMap, fs};
use serde::Deserialize;
use regex::Regex;
use self::rusqlite::functions::FunctionFlags;
use std::sync::Arc;
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

//TODO: Prepare statements and pass around a DB connection object as in https://tedspence.com/investigating-rust-with-sqlite-53d1f9a41112

//TODO: Fix
// for later use, when regex matching is used for text
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
    pub fn new(did: i32) -> CardFilter<'a> {
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
        // let s = omni.clone().to_lowercase();
        cf.fi = CardFilter::parse_omni(omni.as_str());
        // for (key, value) in fi {
        //     match key {
        //         "cmc" => { cf. = value; }
        //         _ => {}
        //     }
        // }

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
                = text(hm) / color(hm) / ctype(hm) / cmc(hm) / color_identity(hm) / power(hm) / toughness(hm) / name(hm)

                rule cmc(hm: &mut std::collections::HashMap<&str, String>)
                = "cmc:" value:$number_range() { hm.insert("cmc", String::from(value)); }
                rule name(hm: &mut HashMap<&str, String>)
                = name_alias()? value:ss_values() { hm.insert("name", value); }
                rule text(hm: &mut HashMap<&str, String>)
                = text_alias() ":" value:ss_values() { hm.insert("text", value); }
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
                rule number_range() = ['-' | '>' | '<'] ['0'..='9']+ / ['0'..='9']+ "-"? (['0'..='9']+)?

                rule name_alias() = ("name:" / "n:")
                rule text_alias() = ("text" / "te")
                rule type_alias() = ("type" / "ty")
                rule color_alias() = ("color" / "c")
                rule power_alias() = ("power" / "p")
                rule toughness_alias() = ("toughness" / "t")
                rule color_identity_alias() = ("color_identity" / "coloridentity" / "ci")

                rule word() -> String
                = s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '{' | '}']+) { String::from(s) }
                rule phrase() -> String
                =s:$("\"" (word() / " ")+ "\"" ) {String::from(s) }
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
                // rule types() = ['l' | 'e' | 'p' | 'i' | 's' | 'c' | 'a']
                rule all_separator() = ['|' | '/' | '+' | '&']
                rule or_separator() = ['|' | '/' ]
                rule and_separator() = ['+' | '&' ]
                
            }
        }

        match omni_parser::root(omni, &mut hm) {
            Ok(_) => {}
            Err(_) => { println!("Attempted to run omniparser with incorrect arguments. Resultant hashmap:\n{:?}", hm); }
        }
        
        hm
    }

    pub fn make_filter(&self, general: bool) -> String {
        //TODO: implement the color identity filter for general search.
        // let com = rcomfdid(self.did).unwrap();

        let initial = match general {
            true => { 
                let com = rcomfdid(self.did).unwrap();
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

        //TODO: remove once the database is migrated.
        let SUPERTYPES = ["legendary", "snow"];
        let TYPES = ["enchantment", "creature", "land", "instant", "sorcery", "artifact", "planeswalker"];

        for (key, value) in self.fi.clone() {
            match key {
                "name" => { vs.push(format!("AND (cards.name LIKE \'%{}%\')", value)); }
                "text" => { 
                    let s = value.replace("\"", "");
                    vs.push(format!("AND (cards.card_text LIKE \'%{}%\')", s)); 
                }
                "color" => { 
                    let cgs = value.split("|"); 
                    let mut vcg = Vec::new();
                    for cg in cgs {
                        // vs.push(String::from("("));
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
                        // vs.push(String::from("("));
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
                    //TODO: Refactor when updating to new database model; all types in one column
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
                            let mut card_type = "subtypes";
                            if SUPERTYPES.contains(&ty) { card_type = "supertypes"; }
                            if TYPES.contains(&ty) { card_type = "types"; }
                            vf.push(format!("{} {} \'%{}%\'", card_type, include, ty));
                        }
                        vtyg.push(format!("({})", vf.join(" AND ")));
                    }
                    vs.push(format!("AND ({})", vtyg.join(" OR ")));
                }
                "cmc" => {
                    // cmc:0-10
                    // cmc:-10
                    // cmc:10-
                    // cmc:10
                    // cmc:>10
                    // cmc:<10
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

const BANNED: [&'static str; 9] = [
    "UGL", "UNH", "UST", "H17", "HHO", "HTR", "THP1", "THP2", "THP3"
];

// impl Set {
//     pub fn new(code: String, name: String) -> Set {
//         Set {code, name}
//     }
// }

//TODO: Write public function to retrieve all cards. Remove layouts scheme, planar, and vanguard

fn ivctoc(vc: Vec<Card>) -> Result<()> {
    let conn = Connection::open("cards.db")?;

    let mut stmt = conn.prepare("insert into cards (
        name, card_text, mana_cost, 
        layout, types, supertypes, 
        subtypes, color_identity, related_cards, 
        cmc, power, toughness)
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)")?;

    for c in vc {
        let mut rc = Vec::new();
        for i in 0..c.related_cards.len() {
            if c.related_cards[i] != c.name {
                rc.push(c.related_cards[i].clone());
            }
        }

        match stmt.insert(&[c.name, c.text, c.mana_cost, 
            c.layout, c.types.join("|"), c.supertypes.join("|"), 
            c.subtypes.join("|"), c.color_identity.join("|"), 
            rc.join("|"), (c.cmc as i8).to_string(),
            c.power, c.toughness]) {
                Ok(_) => {},
                Err(_) => continue,
            };
            
            // .unwrap_or_else(|error| {
                    // panic!("Error: {:?}", error);
        // });
    }
    
    Ok(())
}

fn is (s: Set) -> Result<()> {
    let conn = Connection::open("cards.db")?;

    let mut stmt = conn.prepare("insert into sets (code, name)
        values (?1, ?2)")?;
    
    stmt.insert(&[s.code, s.name])?;

    Ok(())
}

fn icntodc(c: String, did: usize) -> Result<()> {
    let conn = Connection::open("cards.db")?;

    let mut stmt = conn.prepare("insert into deck_contents (card_name, deck)
        values (?1, ?2)")?;
    
    stmt.insert(&[c, did.to_string()])?;

    Ok(())
}

fn ideck(n: String, c: Card) -> Result<()> {
    let conn = Connection::open("cards.db")?;

    let mut stmt = conn.prepare("insert into decks (name, commander, deck_type)
        values (?1, ?2, ?3)")?;
    
    stmt.insert(&[n, c.name, String::from("Commander")])?;

    Ok(())
}

fn stovs(ss: String) -> Vec<String> {
    let mut vs = Vec::new();

    for s in ss.split('|') {
        vs.push(String::from(s));
    }
    vs
}

pub fn rvcftext(mut text: String, did: i32) -> Result<Vec<Card>> {
    let conn = Connection::open("cards.db")?;

    text.insert(0, '%');
    text.push('%');
    // let mut stmt = conn.prepare("")?;

    if did < 0 {
        let mut stmt = conn.prepare("
            SELECT 
                name, 
                card_text, 
                mana_cost,
                layout, 
                types, 
                supertypes, 
                subtypes, 
                color_identity, 
                related_cards, 
                power, 
                toughness, 
                cmc
            FROM `cards`
            WHERE card_text LIKE ?
            ORDER BY name;")?;
        let cards = stmt.query_map(params![text], |row| {
            Ok(Card {
                name: row.get(0)?,
                text: row.get(1)?,
                mana_cost: row.get(2)?,
                layout: row.get(3)?,
                types: stovs(row.get(4)?),
                supertypes: stovs(row.get(5)?),
                subtypes: stovs(row.get(6)?),
                color_identity: stovs(row.get(7)?),
                related_cards: stovs(row.get(8)?),
                power: row.get(9)?,
                toughness: row.get(10)?,
                cmc: row.get(11)?,
            })
        })?.collect();

        cards
    } else {
        let mut stmt = conn.prepare("
        SELECT 
            name, 
			card_text, 
			mana_cost,
            layout, 
			types, 
			supertypes, 
            subtypes, 
			color_identity, 
			related_cards, 
            power, 
			toughness, 
			cmc
        FROM 'cards'
        INNER JOIN 'deck_contents'
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = ?
        AND cards.card_text LIKE ?
        ORDER BY cards.name;")?;
        let cards = stmt.query_map(params![did, text], |row| {
            Ok(Card {
                name: row.get(0)?,
                text: row.get(1)?,
                mana_cost: row.get(2)?,
                layout: row.get(3)?,
                types: stovs(row.get(4)?),
                supertypes: stovs(row.get(5)?),
                subtypes: stovs(row.get(6)?),
                color_identity: stovs(row.get(7)?),
                related_cards: stovs(row.get(8)?),
                power: row.get(9)?,
                toughness: row.get(10)?,
                cmc: row.get(11)?,
            })
        })?.collect();

        cards
    }
}

pub fn rvcfname(mut name: String, did: i32) -> Result<Vec<Card>> {
    let conn = Connection::open("cards.db")?;

    name.insert(0, '%');
    name.push('%');

    if did < 0 {
        let mut stmt = conn.prepare("
            SELECT 
                name, 
                card_text, 
                mana_cost,
                layout, 
                types, 
                supertypes, 
                subtypes, 
                color_identity, 
                related_cards, 
                power, 
                toughness, 
                cmc
            FROM `cards`
            WHERE name LIKE ?
            ORDER BY name;")?;
        let cards = stmt.query_map(params![name], |row| {
            Ok(Card {
                name: row.get(0)?,
                text: row.get(1)?,
                mana_cost: row.get(2)?,
                layout: row.get(3)?,
                types: stovs(row.get(4)?),
                supertypes: stovs(row.get(5)?),
                subtypes: stovs(row.get(6)?),
                color_identity: stovs(row.get(7)?),
                related_cards: stovs(row.get(8)?),
                power: row.get(9)?,
                toughness: row.get(10)?,
                cmc: row.get(11)?,
            })
        })?.collect();

        cards
    } else {
        let mut stmt = conn.prepare("
        SELECT 
            name, 
			card_text, 
			mana_cost,
            layout, 
			types, 
			supertypes, 
            subtypes, 
			color_identity, 
			related_cards, 
            power, 
			toughness, 
			cmc
        FROM 'cards'
        INNER JOIN 'deck_contents'
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = ?
        AND cards.name LIKE ?
        ORDER BY cards.name;")?;
        let cards = stmt.query_map(params![did, name], |row| {
            Ok(Card {
                name: row.get(0)?,
                text: row.get(1)?,
                mana_cost: row.get(2)?,
                layout: row.get(3)?,
                types: stovs(row.get(4)?),
                supertypes: stovs(row.get(5)?),
                subtypes: stovs(row.get(6)?),
                color_identity: stovs(row.get(7)?),
                related_cards: stovs(row.get(8)?),
                power: row.get(9)?,
                toughness: row.get(10)?,
                cmc: row.get(11)?,
            })
        })?.collect();

        cards
    }

    // let mut cs = Vec::new();

    // for c in card_iter {
    //     cs.push(c);
    // }
    
    // cards
}

pub fn import_deck(filename: String, deck_id: usize) -> Result<()> {
    let contents = fs::read_to_string(filename)
        .expect("Could not read file");

    for line in contents.lines() {
        icntodc(String::from(line), deck_id)?;
    }
    Ok(())
}

pub fn create_db() -> Result<()> {
    let conn = Connection::open("cards.db")?;

    conn.execute(
        "create table if not exists sets (
            id integer primary key,
            code text not null unique, 
            name text not null unique
        )", NO_PARAMS,
    )?;

    //TODO: Add reference to rulings, legalities, and side
    conn.execute(
        "create table if not exists cards (
            id integer primary key,
            name text not null unique,
            card_text text,
            mana_cost text not null,
            layout text not null,
            types text not null,
            supertypes text,
            subtypes text,
            color_identity text,
            related_cards text,
            power text,
            toughness text,
            cmc integer not null
        )", NO_PARAMS
    )?;

    //TODO: Add notes, cost, and date_cost_calculated
    conn.execute(
        "create table if not exists decks (
            id integer primary key,
            name text not null,
            commander text not null,
            deck_type text not null,
            foreign key (commander) references cards(name))"
            , NO_PARAMS)?;

    conn.execute(
        "create table if not exists deck_contents (
            id integer primary key,
            card_name text not null,
            deck integer not null,
            tags text,
            foreign key (deck) references decks(id))"
            , NO_PARAMS
    )?;
    Ok(())
}

pub fn full_pull() -> Result<()> {
    println!("Beginning full update of card database.");
    let conn = Connection::open("cards.db")?;
    
    let mut stmt = conn.prepare("select s.code, s.name from sets s;")?;
    let si: Vec<Set> = stmt.query_map(NO_PARAMS, |row| {
        Ok(
            Set {
                code: row.get(0)?,
                name: row.get(1)?,
            }
        )
    })?.filter_map(Result::ok).collect();
    println!("Retrived {} existing sets from the database.", si.len());

    let so = crate::network::rvs().unwrap();
    
    let sd = so
        .iter()
        .filter(|s| !si.contains(s) && !BANNED.contains(& s.code.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    println!("There are {} sets missing from the database.", sd.len());

    for s in sd {
        println!("Found set '{}' missing. Retrieving cards now.", s.name);
        let vc = crate::network::rcs(&s);
        ivctoc(vc)?;
        println!("Inserted all cards in {}", s.name);
        is(s)?;
    }
    Ok(())
}

pub fn rvcfdid(did: i32) -> Result<Vec<Card>> {
    let conn = Connection::open("cards.db")?;

    let mut stmt = conn.prepare("
        SELECT 
            name, 
			card_text, 
			mana_cost,
            layout, 
			types, 
			supertypes, 
            subtypes, 
			color_identity, 
			related_cards, 
            power, 
			toughness, 
			cmc
        FROM `cards`
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = ?
        ORDER BY name;")?;

    
    let cards = stmt.query_map(params![did], |row| {
        Ok(Card {
            name: row.get(0)?,
            text: row.get(1)?,
            mana_cost: row.get(2)?,
            layout: row.get(3)?,
            types: stovs(row.get(4)?),
            supertypes: stovs(row.get(5)?),
            subtypes: stovs(row.get(6)?),
            color_identity: stovs(row.get(7)?),
            related_cards: stovs(row.get(8)?),
            power: row.get(9)?,
            toughness: row.get(10)?,
            cmc: row.get(11)?,
        })
    })?.collect();

    cards
}

pub fn rcomfdid(did: i32) -> Result<Card> {
    let conn = Connection::open("cards.db")?;
    let mut stmt = conn.prepare("SELECT * FROM decks WHERE id = ?;")?;

    let deck = stmt.query_row(params![did], |row| {
        
        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: rcfn(row.get(2)?)?,
        })
    })?;

    let mut stmt = conn.prepare("
    SELECT 
        name, 
        card_text, 
        mana_cost,
        layout, 
        types, 
        supertypes, 
        subtypes, 
        color_identity, 
        related_cards, 
        power, 
        toughness, 
        cmc
    FROM cards WHERE name = ?;")?;

    stmt.query_row(params![deck.commander.name], |row| {
        Ok( Card {
            name: row.get(0)?,
            text: row.get(1)?,
            mana_cost: row.get(2)?,
            layout: row.get(3)?,
            types: stovs(row.get(4)?),
            supertypes: stovs(row.get(5)?),
            subtypes: stovs(row.get(6)?),
            color_identity: stovs(row.get(7)?),
            related_cards: stovs(row.get(8)?),
            power: row.get(9)?,
            toughness: row.get(10)?,
            cmc: row.get(11)?,
        })
    })
}

pub fn rcfn(name: String) -> Result<Card> {
    let conn = Connection::open("cards.db")?;
    let mut stmt = conn.prepare("
    SELECT 
        name, 
        card_text, 
        mana_cost,
        layout, 
        types, 
        supertypes, 
        subtypes, 
        color_identity, 
        related_cards, 
        power, 
        toughness, 
        cmc
    FROM cards WHERE name = ?;")?;
    stmt.query_row(params![name], |row| {
        Ok( Card {
            name: row.get(0)?,
            text: row.get(1)?,
            mana_cost: row.get(2)?,
            layout: row.get(3)?,
            types: stovs(row.get(4)?),
            supertypes: stovs(row.get(5)?),
            subtypes: stovs(row.get(6)?),
            color_identity: stovs(row.get(7)?),
            related_cards: stovs(row.get(8)?),
            power: row.get(9)?,
            toughness: row.get(10)?,
            cmc: row.get(11)?,
        })
    })
}

pub fn rvd () -> Result<Vec<Deck>> {
    let conn = Connection::open("cards.db")?;
    let mut stmt = conn.prepare("SELECT * FROM decks;")?;

    let decks = stmt.query_map(NO_PARAMS, |row| {
        
        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: rcfn(row.get(2)?)?,
        })
    })?.collect();

    decks
}

pub fn rdfdid(id: i32) -> Result<Deck> {
    let conn = Connection::open("cards.db")?;
    let mut stmt = conn.prepare("SELECT * FROM decks WHERE id = ?;")?;
    stmt.query_row(params![id], |row| {
        Ok( Deck {
            name: row.get(1)?,
            commander: rcfn(row.get(2)?)?,
            id: row.get(0)?,
        })
    })
}

pub fn db_test(s: &str) -> Result<Vec<Card>> {
    let conn = Connection::open("cards.db")?;
    let _a = add_regexp_function(&conn);
    let qs = format!("
        SELECT 
            name, 
            card_text, 
            mana_cost,
            layout, 
            types, 
            supertypes, 
            subtypes, 
            color_identity, 
            related_cards, 
            power, 
            toughness, 
            cmc
        FROM `cards`
        {}
        ORDER BY name;", s);

    let mut stmt = conn.prepare(&qs)?;

        let cards = stmt.query_map(NO_PARAMS, |row| {
            Ok(Card {
                name: row.get(0)?,
                text: row.get(1)?,
                mana_cost: row.get(2)?,
                layout: row.get(3)?,
                types: stovs(row.get(4)?),
                supertypes: stovs(row.get(5)?),
                subtypes: stovs(row.get(6)?),
                color_identity: stovs(row.get(7)?),
                related_cards: stovs(row.get(8)?),
                power: row.get(9)?,
                toughness: row.get(10)?,
                cmc: row.get(11)?,
            })
        })?.collect();

        cards
}

pub fn rvcfcf(cf: CardFilter, general: bool) -> Result<Vec<Card>> {
    let conn = Connection::open("cards.db")?;
    let _a = add_regexp_function(&conn);
    let qs = format!("
        SELECT 
            name, 
            card_text, 
            mana_cost,
            layout, 
            types, 
            supertypes, 
            subtypes, 
            color_identity, 
            related_cards, 
            power, 
            toughness, 
            cmc
        FROM `cards`
        {}
        ORDER BY name;", cf.make_filter(general));
    // println!("{}", qs);

    let mut stmt = conn.prepare(& qs)?;

        let cards = stmt.query_map(NO_PARAMS, |row| {
            Ok(Card {
                name: row.get(0)?,
                text: row.get(1)?,
                mana_cost: row.get(2)?,
                layout: row.get(3)?,
                types: stovs(row.get(4)?),
                supertypes: stovs(row.get(5)?),
                subtypes: stovs(row.get(6)?),
                color_identity: stovs(row.get(7)?),
                related_cards: stovs(row.get(8)?),
                power: row.get(9)?,
                toughness: row.get(10)?,
                cmc: row.get(11)?,
            })
        })?.collect();

        // if cards == Error { println!("No cards found");  }

        cards
}
