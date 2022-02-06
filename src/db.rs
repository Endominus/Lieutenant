extern crate rusqlite;
extern crate regex;
extern crate pest;

use crate::util::{CardLayout, CardStat, Card, Deck, CommanderType};
use crate::network::{rvjc, rextcostfcn};

use self::rusqlite::{params, Connection};
use std::{collections::HashMap, convert::TryInto, sync::Mutex};
use rusqlite::{Row, named_params, Result, Error};
use serde::{Deserialize, Serialize};
use regex::Regex;
use self::rusqlite::functions::FunctionFlags;
use std::sync::Arc;
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
use std::{thread, time};
use chrono::{Datelike, Utc, Duration, TimeZone};
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

// pub struct DbContext<'a> {
//     conn: Connection,
//     stmts: HashMap<&'a str, Statement<'a>>
// }

use crate::network::rcostfcn;
use crate::util::{SortOrder, DefaultFilter};

const DB_FILE: &str = "lieutenant.db";

#[derive(Default)]
pub struct CardFilter {
    pub did: i32,
    color: String,
    df: DefaultFilter, 
    so: SortOrder,
}

#[derive(Parser)]
#[grammar = "omni.pest"] // relative to src
struct OmniParser;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Set {
    pub code: String,
    pub name: String,
    #[serde(alias = "releaseDate")]
    pub date: String,
    #[serde(alias = "type")]
    pub set_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportCard {
    pub name: String,
    pub tags: Option<String>,
}

enum ParseMode {
    Text,
    Tags,
    Color
}

#[derive(PartialEq, Eq)]
enum FilterField {
    Name,
    Text,
    Tag,
    Type,
    CMC,
    Power,
    Toughness,
    Color,
    Identity,
    None
}

impl PartialEq for ImportCard {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl CardFilter {
    pub fn new() -> CardFilter {
        CardFilter::default()
    }

    pub fn from(did: i32, color: &String, default_filter: DefaultFilter, sort_order: SortOrder) -> CardFilter {
        CardFilter { did, color: color.clone(), df: default_filter, so: sort_order }
    }

    pub fn make_query(&self, general: bool, omni: &str) -> String {
        let initial = match general {
            true => { 
                let mut colors = String::from("WUBRG");
                for c in self.color.chars() {
                    colors = colors.replace(c, "");
                }
                let ci = match colors.len() {
                    0 => { String::from("1=1") }
                    _ => { format!("color_identity REGEXP \'^[^{}]*$\'", &colors) }
                };
                format!("
LEFT OUTER JOIN deck_contents
ON cards.name = deck_contents.card_name
AND deck_contents.deck = {}
WHERE {}", self.did, ci) 
            }
            false => { 
                format!("
INNER JOIN deck_contents
ON cards.name = deck_contents.card_name
WHERE deck_contents.deck = {}", self.did) 
            }
        };

        let mut filters = String::new();
        let mut ordering = match self.so {
            SortOrder::NameAsc => "ORDER BY name ASC;".into(),
            SortOrder::NameDesc => "ORDER BY name DESC;".into(),
            SortOrder::CmcAsc => "ORDER BY cmc ASC;".into(),
            SortOrder::CmcDesc => "ORDER BY cmc DESC;".into(),
        };

        match OmniParser::parse(Rule::input, omni) {
            Ok(mut pairs) => {
                let enclosed = pairs.next().unwrap();
                let tokens = enclosed.into_inner();
                for rule in tokens {
                    if rule.as_rule() == Rule::sort {
                        ordering = CardFilter::helper(rule, &FilterField::None);
                    } else {
                        let s = CardFilter::helper(rule, &FilterField::None);
                        filters += &String::from(format!("\nAND ({})", s));
                    }
                }
            }
            Err(_) => {
                let default = match self.df {
                    DefaultFilter::Name => "name",
                    DefaultFilter::Text => "card_text",
                };
                let error = omni.replace("\"", "");
                filters += &String::from(format!("\nAND {default} LIKE \"%{error}%\""));
            }
        }

        String::from(format!("\n{initial}{filters}\n{ordering}"))
    }

    fn helper(p: Pair<Rule>, mode: &FilterField) -> String {
        let mut s = String::new();
        match p.as_rule() {
            Rule::name => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Name);
                }
            },
            Rule::text => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Text);
                }
            },
            Rule::ctyp => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Type);
                }
            },
            Rule::tag => {
                //TODO: Check if brackets are necessary here.
                let i = p.into_inner();
                for r in i {
                    if r.as_str() == "!" {
                        s += "tags IS NULL";
                    } else if r.as_rule() == Rule::text_token {
                        s += "tags IS NOT NULL AND ";
                        s += &CardFilter::helper(r, &FilterField::Tag);
                    } else {
                        s += &CardFilter::helper(r, &FilterField::Tag);
                    }
                }
            },
            Rule::cmc => {
                let r = p.into_inner().next().unwrap();
                s += &CardFilter::helper(r, &FilterField::CMC);
            },
            Rule::power => {
                let r = p.into_inner().next().unwrap();
                s += &CardFilter::helper(r, &FilterField::Power);
            },
            Rule::toughness => {
                let r = p.into_inner().next().unwrap();
                s += &CardFilter::helper(r, &FilterField::Toughness);
            },
            Rule::color => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Color);
                }
            },
            Rule::identity => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Identity);
                }
            },
            Rule::rarity => {
                let i = p.into_inner();
                let mut flag = false;
                for r in i {
                    if flag {
                        s.push_str(" OR ");
                    } else {
                        flag = true;
                    }
                    s += &CardFilter::helper(r, &FilterField::None);
                }
    
            },
            Rule::sort => {
                let mut a: String = p.as_str().strip_prefix("sort:").unwrap().into();
                let order = if a.remove(0) == '-' { "DESC" } else { "ASC" };
                let field = if a.remove(0) == 'c' { "cmc" } else { "name" };
                s = format!("ORDER BY {field} {order};");
            }
            Rule::text_token => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, mode);
                }
            },
            Rule::bracketed_text => {
                s.push('(');
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, mode);
                }
                s.push(')');
            },
            Rule::color_token => {
                let i = p.into_inner();
                let mut flag = false;
                for r in i {
                    if flag {
                        s.push_str(" AND ");
                    } else {
                        flag = true;
                    }
                    s += &CardFilter::helper(r, mode);
                }
            }
            Rule::word | Rule::phrase => {
                let mut a = p.as_str().replace("\"", "");
                let mut flag = " ";
                if let Some(i) = p.into_inner().next() {
                    if i.as_rule() == Rule::negation {
                        a = String::from(a.trim_start_matches('!'));
                        flag = " NOT ";
                    }
                }
    
                if mode == &FilterField::Type {
                    match a.as_str() {
                        "a" => a = String::from("artifact"),
                        "c" => a = String::from("creature"),
                        "e" => a = String::from("enchantment"),
                        "i" => a = String::from("instant"),
                        "l" => a = String::from("legendary"),
                        "p" => a = String::from("planeswalker"),
                        "s" => a = String::from("sorcery"),
                        "per" => { //easiest thing to do is just return from here. Inelegant, though.
                            if flag == " " {
                                return String::from("types NOT LIKE \'%instant%\' AND types NOT LIKE \'%sorcery%\'")
                            } else {
                                return String::from("types LIKE \'%instant%\' OR types LIKE \'%sorcery%\'")
                            }
                        },
                        _ => {},
    
                    }
                }
    
                let (field, comparison, capture) = match mode {
                    FilterField::Name => ("name", "LIKE", format!("\"%{a}%\"")),
                    FilterField::Text => ("card_text", "LIKE", format!("\"%{a}%\"")),
                    FilterField::Type => ("types", "LIKE", format!("\"%{a}%\"")),
                    FilterField::Tag => ("tags", "REGEXP", format!(r#"'\|?{a}(?:$|\|)'"#)),
                    _ => ("", "", String::new())
                };
    
                s = format!("{field}{flag}{comparison} {capture}")
            },
            Rule::separator => {
                let sep = p.into_inner().next().unwrap();
                if sep.as_rule() == Rule::and_separator {
                        s.push_str(" AND ");
                } else {
                        s.push_str(" OR ");
                }
            },
            Rule::or_separator => {
                s = " OR ".into();
            },
            Rule::number_range => {
                let field = match mode {
                    FilterField::CMC => "cmc",
                    FilterField::Power => "power",
                    FilterField::Toughness => "toughness",
                    _ => "",
                };
    
                let range = p.as_str();
                if range == "*" {
                    s = match mode {
                        FilterField::CMC => String::from("mana_cost LIKE \'%X%\'"),
                        FilterField::Power => String::from("power LIKE \'%*%\'"),
                        FilterField::Toughness => String::from("toughness LIKE \'%*%\'"),
                        _ => String::new(),
                    }
                } else if range.contains('-') {
                    let (a, b) = range.split_once('-').unwrap();
                    s = format!("{field} >= {} AND {field} <= {}", a, b);
                } else if range.contains("..") {
                    let (a, b) = range.split_once("..").unwrap();
                    s = format!("{field} >= {} AND {field} <= {}", a, b);
                } else if range.starts_with('>') {
                    s = format!("{field} > {}", range.strip_prefix('>').unwrap());
                } else if range.starts_with('<') {
                    s = format!("{field} < {}", range.strip_prefix('<').unwrap());
                } else {
                    s = format!("{field} = {}", range);
                }
            },
            Rule::color_val => {
                let mut a = p.as_str();
                let mut req = ">";
                if let Some(i) = p.into_inner().next() {
                    if i.as_rule() == Rule::negation {
                        req = "=";
                        a = a.trim_start_matches('!');
                    }
                }
                
                let field = match mode {
                    FilterField::Color => {
                        if a == "c" {
                            if req == ">" {
                                return String::from("mana_cost NOT REGEXP \'[WUBRG]+\'")
                            } else {
                                return String::from("mana_cost REGEXP \'[WUBRG]+\'")
                            }
                        }
                        "mana_cost"
                    },
                    FilterField::Identity => {
                        if a == "c" {
                            if req == ">" {
                                return String::from("color_identity = \'\'")
                            } else {
                                return String::from("color_identity != \'\'")
                            }
                        }
                        "color_identity"
                    },
                    _ => "",
                };
    
                s = format!("instr({}, \'{}\') {} 0", field, a.to_uppercase(), req);
            },
            Rule::rarity_val => {
                let mut a = p.as_str();
                let req = if a.starts_with('!') {
                    a = a.trim_start_matches('!');
                    "!="
                } else { "=" };
    
                let val = match a {
                    "c" => "common",
                    "u" => "uncommon",
                    "r" => "rare",
                    "m" => "mythic",
                    _ => ""
                };
    
                s = format!("rarity {req} {val}");
            }
            Rule::negation => {
                s = "not ".into();
            }
            _ => {}
        }
    
        s
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

fn parse_args(column: &str, mode: ParseMode, items: &String) -> String {
    let mut v_or_conditions = Vec::new();
    
    let p = match &mode {
        ParseMode::Text =>  { "{:col} {:req} \"%{:item}%\"" }
        ParseMode::Tags =>  { r#"{:col} {:req} '\|?{:item}(?:$|\|)'"# }
        ParseMode::Color => { "instr({:col}, '{:item}') {:req} 0" }
    };
    
    let groups = items.split("|");
    for mut group in groups {
        let mut v_and_conditions = Vec::new();
        let negation = group.get(0..1);
        let mut _g = String::new();
        let (req, items): (&str, Vec<&str>) = match &mode {
            ParseMode::Text => {
                match negation {
                    Some("!") => { group = group.get(1..).unwrap(); ("NOT LIKE", group.split("&").collect()) }
                    Some(_) => { ("LIKE", group.split("&").collect()) }
                    None => { continue; }
                }
            }
            ParseMode::Tags => {
                if group == "!" {
                    v_or_conditions.push(format!(r#"tags IS NULL"#));
                    continue;
                }
                v_and_conditions.push(format!(r#"tags IS NOT NULL"#));
                match negation {
                    Some("!") => { 
                        group = group.get(1..).unwrap(); 
                        ("NOT REGEXP", group.split("&").collect()) 
                    }
                    Some(_) => { ("REGEXP", group.split("&").collect()) }
                    None => { continue; }
                }
            }
            ParseMode::Color => {
                match negation {
                    Some("!") => { 
                        _g = group.get(1..).unwrap().to_uppercase(); 
                        ("=", _g.split_inclusive(|_c| true).collect()) }
                    Some(_) => { 
                        _g = group.to_uppercase(); 
                        (">", _g.split_inclusive(|_c| true).collect()) }
                    None => { continue; }
                }
            }
        };

        for item in items {
            let s = p
                .replace("{:col}", column)
                .replace("{:req}", req)
                .replace("{:item}", item);
            v_and_conditions.push(s);
        }
        v_or_conditions.push(format!("({})", v_and_conditions.join(" AND ")));
    }

    format!("AND ({})", v_or_conditions.join(" OR "))
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
            name text not null unique,
            date text not null,
            set_type text NOT NULL
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
            rarity,
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

pub fn updatedb(conn: &Connection, mut sets: Vec<Set>) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(sets);")?;
    sets.sort_by(|a, b| a.date.cmp(&b.date));
    let mut cols = Vec::new();
    match stmt.query([]) {
        Ok(mut results) => { 
            while let Some(row) = results.next()? {
                cols.push(row.get::<usize, String>(1)?);
            }
        }
        Err(e) => { println!("Error!\n{:?}", e); }
    };
    if !cols.contains(&String::from("date")) {
        let mut stmt = conn.prepare("ALTER TABLE sets ADD COLUMN date text INTEGER NOT NULL")?;
        stmt.execute([]).unwrap();
    }
    if !cols.contains(&String::from("set_type")) {
        let mut stmt = conn.prepare("ALTER TABLE sets ADD COLUMN set_type text NOT NULL")?;
        stmt.execute([]).unwrap();
    }

    stmt = conn.prepare("SELECT * FROM sets;")?;
    let rows: Result<Vec<Set>> = stmt.query_map([], |row| 
        Ok(Set { 
            code: row.get(1)?,
            name: row.get(2)?,
            date: row.get(3)?,
            set_type: row.get(4)?,
        }
    ))?.collect();
    let existing_sets = rows?;
    let mut set_stmt = conn.prepare("INSERT INTO sets (code, name, date, set_type) VALUES (:code, :name, :date, :set_type);")?;

    let now = Utc::now();
    let date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());

    for set in sets {
        if !existing_sets.contains(&set) && set.date <= date {
            println!{"Set {} was printed on {}, which is apparently less than {}", set.name, set.date, date};
            println!("Adding {} to existing sets.", set.name);

            let vjc = rvjc(&set.code).unwrap();
            let (success, failure) = ivcfjsmap(conn, vjc)?;
            println!("Added {} cards, with {} not added.", success, failure);

            set_stmt.execute(named_params!{
                ":code": set.code, 
                ":name": set.name, 
                ":date": set.date,
                ":set_type": set.set_type 
            })?;
        }
    }

    Ok(())
}

pub fn ucfsqlite(conn_primary: &Connection, conn_secondary: &Connection,) -> Result<()> {
    let mut stmt = conn_primary.prepare("UPDATE cards SET rarity = :rarity WHERE name = :name;")?;

    let mut stmt_second = conn_secondary.prepare("SELECT name, faceName, rarity FROM cards;")?;
    let mut result = HashMap::new();
    let _a: Result<Vec<()>> = stmt_second.query_map(
        [], 
        |row| { 
            let name = match row.get::<usize, String>(1) {
                Ok(s) => { s }
                _ => { row.get::<usize, String>(0)? }
            };
            let rarity = row.get::<usize, String>(2)?;
            if rarity != "special".to_string() && rarity != "bonus".to_string() {
                result.insert(name, rarity); 
            }
            Ok(()) 
        })?.collect();

    let (mut success, mut failure) = (0, 0);

    println!("Generated card array. {} total cards.", result.len());
    conn_primary.execute_batch("BEGIN TRANSACTION;")?;

    for (name, rarity) in result {

        match stmt.execute(named_params!{
            ":name": name,
            ":rarity": rarity,
        }) {
            Ok(_) => { success += 1; },
            Err(_) => { 
                failure += 1;
             },
        }
    }
    
    conn_primary.execute_batch("COMMIT TRANSACTION;")?;

    println!("{} cards changed successfully. {} failures.", success, failure);

    Ok(())
}

pub fn ivcfjsmap(conn: &Connection, vjc: Vec<JsonCard>) -> Result<(usize, usize)> {
    let mut stmt = conn.prepare("INSERT INTO cards (
        name, mana_cost, cmc, types, card_text, power, toughness, loyalty, color_identity, related_cards, layout, side, legalities, rarity
    ) VALUES (
            :name, :mana_cost, :cmc, :types, :card_text, :power, :toughness, :loyalty, :color_identity, :related_cards, :layout, :side, :legalities, :rarity
    )")?;
    let (mut success, mut failure) = (0, 0);

    print!("Found {} cards. ", vjc.len());
    conn.execute_batch("BEGIN TRANSACTION;")?;

    for c in vjc {
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
            ":rarity": c.rarity,
        }) {
            Ok(_) => { success += 1; },
            Err(_) => { 
                // Usually an unset or duplicate cards
                failure += 1;
             },
        }
    }

    
    let illegal = conn.execute("DELETE
FROM cards
WHERE legalities = \"\"", [])?;
    
    println!("Added {} to database. {} were not added (duplicate or illegal).", success, failure+illegal);
    
    conn.execute_batch("COMMIT TRANSACTION;")?;
    
    Ok((success, failure))
}

pub fn ictodc(conn: &Connection, c: &Card, did: i32) -> Result<Vec<Card>> {
    let mut r = Vec::new();
    let mut stmt = conn.prepare("INSERT INTO deck_contents (card_name, deck) VALUES (:card_name, :deck_id)")?;
    stmt.execute(named_params!{":card_name": c.name, ":deck_id": did as u32} )?;
    r.push(rcfn(conn, &c.name, None).unwrap());
    
    match &c.lo {
        CardLayout::Flip(_, n) | 
        CardLayout::Split(_, n) | 
        CardLayout::ModalDfc(_, n) | 
        CardLayout::Aftermath(_, n) | 
        CardLayout::Adventure(_, n) | 
        CardLayout::Transform(_, n) => { 
            stmt.execute(named_params!{":card_name": n, ":deck_id": did as u32} )?;
            r.push(rcfn(conn, n, None).unwrap());
        }
        CardLayout::Meld(s, n, m) => { 
            if s == &'b' {  
                stmt.execute(named_params!{":card_name": n, ":deck_id": did as u32} )?;
                r.push(rcfn(conn, n, None).unwrap()); 
                stmt.execute(named_params!{":card_name": m, ":deck_id": did as u32} )?;
                r.push(rcfn(conn, m, None).unwrap());
            } else {
                let names: Vec<String> =  rvcfdid(conn, did, SortOrder::NameAsc).unwrap().iter().map(|c| c.to_string()).collect();
                if names.contains(&n) {  
                    stmt.execute(named_params!{":card_name": m, ":deck_id": did as u32} )?;
                    r.push(rcfn(conn, m, None).unwrap());
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
    let mut stmt = conn.prepare("UPDATE deck_contents 
        SET tags = :tags
        WHERE card_name = :name
        AND deck = :did;").unwrap();

    let mut card = rcfndid(conn, c, did).unwrap();
    let cc = if t.eq(&String::from("main")) || t.eq(&String::from("main")) {
        match &card.lo {
            CardLayout::Adventure(_, n) | 
            CardLayout::Aftermath(_, n) | 
            CardLayout::Flip(_, n) | 
            CardLayout::ModalDfc(_, n) | 
            CardLayout::Split(_, n) | 
            CardLayout::Transform(_, n) => { 
                Some(rcfndid(conn, n, did).unwrap())
            },
            _ => { None }
        }
    } else { None };

    let tags = if card.tags.contains(&t) {
        card.tags.remove(card.tags.iter().position(|x| x == t).unwrap());
        if card.tags.is_empty() {
            None
        } else {
            Some(card.tags.join("|"))
        }
    } else {
        card.tags.push(t.clone());
        card.tags.sort();
        Some(card.tags.join("|"))
    };

    stmt.execute(named_params!{":tags": tags, ":name": c, ":did": did}).unwrap();
    if let Some(mut cc) = cc {
        if !card.tags.contains(&t) && cc.tags.contains(&t) { 
            cc.tags.remove(cc.tags.iter().position(|x| x == t).unwrap());
        } else if card.tags.contains(&t) && !cc.tags.contains(&t) {
            cc.tags.push(t.clone());
        }
        let t = if cc.tags.is_empty() { None } else { Some(cc.tags.join("|")) };
        stmt.execute(named_params!{":tags": t, ":name": cc.name, ":did": did}).unwrap();
    }
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
    match c2 {
        Some(c2) => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, commander2, deck_type) VALUES (:name, :commander, :commander2, :deck_type);").unwrap();
            stmt.execute(named_params!{":name": n, ":commander": c, ":commander2": c2, ":deck_type": t} ).unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, &c, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();
            ttindc(conn, c, &String::from("main"), rid.try_into().unwrap());
        
            let com = rcfn(conn, &c2, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();
            ttindc(conn, &c2, &String::from("main"), rid.try_into().unwrap());

            Ok(rid.try_into().unwrap())
            
        }
        None => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, deck_type) VALUES (:name, :commander, :deck_type);").unwrap();
            stmt.execute(named_params!{":name": n, ":commander": c, ":deck_type": t} ).unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, &c, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();
            ttindc(conn, c, &String::from("main"), rid.try_into().unwrap());

            Ok(rid.try_into().unwrap())
        }
    }
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
                        (&cards.first().unwrap().name, Some(ss))
                    } else {
                        println!("Valid commander found: {}", c.name);
                        println!("But did you forget to put {} in the deck? It wasn't found.", &ss);
                        (&cards.first().unwrap().name, None) 
                    }
                }
                CommanderType::Invalid  => { 
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
            let c = ic.name.trim().to_string();
            if c.len() == 0 { continue }
            let card = if let Some(i) = c.find(" // ") {
                let c = c.get(0..i).unwrap();
                rcfn(conn, &c.to_string(), None).unwrap()
            } else {
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
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags, rarity, price, date_price_retrieved
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
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags, rarity, price, date_price_retrieved
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
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags, rarity, price, date_price_retrieved
        FROM cards 
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did
        ORDER BY {} {};", sort_on, order);
    

    let mut stmt = conn.prepare(s.as_str())?;
    
    let a = stmt.query_map(named_params!{ ":did": did }, |row| { cfr(row) })?;
    a.collect()
}

pub fn rvcfcf(conn: &Connection, query: &String) -> Result<Vec<Card>> {
    let fields = "cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags, rarity, price, date_price_retrieved";
    let qs = format!("SELECT {}
FROM `cards`
{}", fields, query);

    let mut stmt = conn.prepare(& qs).expect("issue with filter string");

    let cards = stmt.query_map([], |row| {
        cfr(row)
    })?.collect();

    cards
}

pub fn rvcnfcf(conn: &Connection, query: &String) -> Result<Vec<String>> {
    let fields = "name";
    let qs = format!("SELECT {}
FROM cards
{}", fields, query);
    let mut stmt = conn.prepare(& qs).expect("issue with filter string");

    let cards = stmt.query_map([], |row| {
        row.get(0)
    })?.collect();

    cards
}

pub fn rvcnfn(conn: &Connection, n: &String) -> Result<Vec<String>> {
    if n.len() == 0 {
        return Ok(Vec::new())
    }
    let query = format!("
SELECT name
FROM cards
WHERE name LIKE \"%{}%\"
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
                    CardLayout::Adventure(side, rel)    
                }
                "aftermath" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    CardLayout::Aftermath(side, rel)    
                }
                "flip" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    CardLayout::Flip(side, rel)    
                }
                "leveler" => { CardLayout::Leveler }
                "meld" => { 
                    let rel = row.get::<usize, String>(11)?;
                    let rels = rel.split_once("|").unwrap();
                    let (face, transform) = (String::from(rels.0), String::from(rels.1));
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    CardLayout::Meld(side, face, transform)
                }
                "modal_dfc" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    CardLayout::ModalDfc(side, rel)    
                }
                "normal" => { CardLayout::Normal }
                "saga" => { CardLayout::Saga }
                "split" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    CardLayout::Split(side, rel)    
                }
                "transform" => {
                    let rel = row.get::<usize, String>(11)?;
                    let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                    CardLayout::Transform(side, rel)
                }
                _ => { CardLayout::Normal }
            }
         }
        Err(_) => { CardLayout::Normal }
    };
    
    let tags: Vec<String> = match row.get::<usize, String>(13) {
        Ok(a) => { stovs(a) }
        Err(_) => { Vec::new() }
    };

    let price = match row.get(15) {
        Ok(a) => { Some(a) }
        Err(_) => { None }
    };

    let stale = match row.get::<usize, String>(16) {
        Ok(a) => { 
            let date = Utc::today();
            let vs: Vec<u32> = a.split('-').map(|s| s.parse::<u32>().unwrap() ).collect();
            let ret = Utc.ymd(vs[0] as i32, vs[1], vs[2]);

            if date - ret > Duration::days(30) {
                true
            } else {
                false
            }
        }
        Err(_) => { true }
    };

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
        tags,
        rarity: row.get(14)?,
        price,
        stale,
    })
}

pub fn rvicfdid(conn: &Connection, did: i32) -> Result<Vec<ImportCard>> {
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

pub fn upfcn_quick(conn: &Connection, cn: &String) {
    let c = rcfn(conn, cn, None).unwrap();
    let s = match c.lo {
        CardLayout::Adventure(_, rel) 
        | CardLayout::Aftermath(_, rel) 
        | CardLayout::Flip(_, rel) 
        | CardLayout::ModalDfc(_, rel) 
        | CardLayout::Split(_, rel) 
        | CardLayout::Transform(_, rel) => format!("{} // {}", cn, rel),
        _ => cn.clone()
    };

    let price = rcostfcn(&s, c.price).unwrap();

    let _ = conn.execute("UPDATE cards 
        SET price = :price, 
        date_price_retrieved = date()
        WHERE name = :name;", 
        named_params!{":price": price, ":name": cn});
}

pub fn upfcn_detailed(conn: &Connection, c: &Card, odid: Option<i32>) -> Result<Card> {
    let s = match &c.lo {
        CardLayout::Adventure(_, rel) 
        | CardLayout::Aftermath(_, rel) 
        | CardLayout::Flip(_, rel) 
        | CardLayout::ModalDfc(_, rel) 
        | CardLayout::Split(_, rel) 
        | CardLayout::Transform(_, rel) => format!("{} // {}", &c.name, rel),
        _ => c.name.clone()
    };

    let price = rextcostfcn(&s).unwrap();

    conn.execute("UPDATE cards 
        SET price = :price, 
        date_price_retrieved = date()
        WHERE name = :name;", 
        named_params!{":price": price, ":name": &s})?;

    rcfn(conn, &s, odid)
}

// pub fn ucfcn(conn: &Connection, cn: &String, layout: &CardLayout, odid: Option<i32>) -> Result<Card> {
//     let s = match layout {
//         CardLayout::Adventure(_, rel) 
//         | CardLayout::Aftermath(_, rel) 
//         | CardLayout::Flip(_, rel) 
//         | CardLayout::ModalDfc(_, rel) 
//         | CardLayout::Split(_, rel) 
//         | CardLayout::Transform(_, rel) => format!("{} // {}", cn, rel),
//         _ => cn.clone()
//     };
//     let price = rcostfcn(&s).unwrap();

//     conn.execute("UPDATE cards 
//         SET price = :price, 
//         date_price_retrieved = date()
//         WHERE name = :name;", 
//         named_params!{":price": price, ":name": cn})?;

//     rcfn(conn, cn, odid)
// }

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
    let res = rcostfcn(&s, None).unwrap();

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
    pub rarity: String,
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