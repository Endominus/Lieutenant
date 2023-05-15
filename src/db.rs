extern crate pest;
extern crate regex;
extern crate rusqlite;

use crate::network::{rextcostfcn, rcostfcn, rvjc};
use crate::util::views::TagChange;
use crate::util::{Card, CardLayout, CardStat, CommanderType, Deck, DefaultFilter, SortOrder};

use self::rusqlite::functions::FunctionFlags;
use self::rusqlite::{params, Connection};
use regex::Regex;
use rusqlite::{named_params, Error, Result, Row};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, convert::TryInto, sync::Mutex};
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
use chrono::{Datelike, Duration, TimeZone, Utc};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::{thread, time};

const DB_FILE: &str = "lieutenant.db";

#[derive(Default)]
pub struct CardFilter {
    pub did: i32,
    color: String,
    pub df: DefaultFilter,
    pub so: SortOrder,
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
    Color,
}

#[derive(PartialEq, Eq)]
enum FilterField {
    Name,
    Text,
    Tag,
    Type,
    Cmc,
    Power,
    Toughness,
    Color,
    Identity,
    None,
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

    pub fn from(
        did: i32,
        color: &str,
        default_filter: DefaultFilter,
        sort_order: SortOrder,
    ) -> CardFilter {
        CardFilter {
            did,
            color: color.to_string(),
            df: default_filter,
            so: sort_order,
        }
    }

    pub fn make_query(&self, general: bool, omni: &str) -> String {
        let initial = match general {
            true => {
                let mut colors = String::from("WUBRG");
                for c in self.color.chars() {
                    colors = colors.replace(c, "");
                }
                let ci = match colors.len() {
                    0 => String::from("1=1"),
                    _ => {
                        format!("color_identity REGEXP \'^[^{}]*$\'", &colors)
                    }
                };
                format!(
                    "
LEFT OUTER JOIN deck_contents
ON cards.name = deck_contents.card_name
AND deck_contents.deck = {}
WHERE {}",
                    self.did, ci
                )
            }
            false => {
                format!(
                    "
INNER JOIN deck_contents
ON cards.name = deck_contents.card_name
WHERE deck_contents.deck = {}",
                    self.did
                )
            }
        };

        let mut filters = String::new();
        let mut ordering = match self.so {
            SortOrder::NameAsc => "ORDER BY name ASC;".into(),
            SortOrder::NameDesc => "ORDER BY name DESC;".into(),
            SortOrder::CmcAsc => "ORDER BY cmc ASC;".into(),
            SortOrder::CmcDesc => "ORDER BY cmc DESC;".into(),
            SortOrder::PriceAsc => "ORDER BY price ASC;".into(),
            SortOrder::PriceDesc => "ORDER BY price DESC;".into(),
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
                        filters += &format!("\nAND ({})", s);
                    }
                }
            }
            Err(_) => {
                if omni.get(0..1) != Some("/") {
                    let default = match self.df {
                        DefaultFilter::Name => "name",
                        DefaultFilter::Text => "card_text",
                    };
                    let error = omni.replace('\"', "");
                    filters += &format!("\nAND {default} LIKE \"%{error}%\"");
                }
            }
        }

        format!("\n{initial}{filters}\n{ordering}")
    }

    fn helper(p: Pair<Rule>, mode: &FilterField) -> String {
        let mut s = String::new();
        match p.as_rule() {
            Rule::name => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Name);
                }
            }
            Rule::text => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Text);
                }
            }
            Rule::ctyp => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Type);
                }
            }
            Rule::tag => {
                //TODO: Check if brackets are necessary here.
                let i = p.into_inner();
                for r in i {
                    if r.as_str() == "!" {
                        s += "tags IS NULL";
                    } else if r.as_rule() == Rule::text_token {
                        s += "tags IS NOT NULL AND (";
                        s += &CardFilter::helper(r, &FilterField::Tag);
                        s += ")"
                    } else {
                        s += &CardFilter::helper(r, &FilterField::Tag);
                    }
                }
            }
            Rule::cmc => {
                let r = p.into_inner().next().unwrap();
                s += &CardFilter::helper(r, &FilterField::Cmc);
            }
            Rule::power => {
                let r = p.into_inner().next().unwrap();
                s += &CardFilter::helper(r, &FilterField::Power);
            }
            Rule::toughness => {
                let r = p.into_inner().next().unwrap();
                s += &CardFilter::helper(r, &FilterField::Toughness);
            }
            Rule::color => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Color);
                }
            }
            Rule::identity => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, &FilterField::Identity);
                }
            }
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
            }
            Rule::sort => {
                let mut a: String = p.as_str().strip_prefix("sort:").unwrap().into();
                let order = if a.remove(0) == '-' { "DESC" } else { "ASC" };
                // let field = if a.remove(0) == 'c' { "cmc" } else { "name" };
                let field = match a.remove(0) {
                    'c' => "cmc",
                    'p' => "price",
                    _ => "name",
                };
                s = format!("ORDER BY {field} {order};");
            }
            Rule::text_token => {
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, mode);
                }
            }
            Rule::bracketed_text => {
                s.push('(');
                let i = p.into_inner();
                for r in i {
                    s += &CardFilter::helper(r, mode);
                }
                s.push(')');
            }
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
                let mut a = p.as_str().replace('\"', "");
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
                        "per" => {
                            //easiest thing to do is just return from here. Inelegant, though.
                            if flag == " " {
                                return String::from(
                                    "types NOT LIKE \'%instant%\' AND types NOT LIKE \'%sorcery%\'",
                                );
                            } else {
                                return String::from(
                                    "types LIKE \'%instant%\' OR types LIKE \'%sorcery%\'",
                                );
                            }
                        }
                        _ => {}
                    }
                }

                let (field, comparison, capture) = match mode {
                    FilterField::Name => ("name", "LIKE", format!("\"%{a}%\"")),
                    FilterField::Text => ("card_text", "LIKE", format!("\"%{a}%\"")),
                    FilterField::Type => ("types", "LIKE", format!("\"%{a}%\"")),
                    FilterField::Tag => ("tags", "REGEXP", format!(r#"'(?:\||^){a}(?:$|\|)'"#)),
                    _ => ("", "", String::new()),
                };

                s = format!("{field}{flag}{comparison} {capture}")
            }
            Rule::separator => {
                let sep = p.into_inner().next().unwrap();
                if sep.as_rule() == Rule::and_separator {
                    s.push_str(" AND ");
                } else {
                    s.push_str(" OR ");
                }
            }
            Rule::or_separator => {
                s = " OR ".into();
            }
            Rule::number_range => {
                let field = match mode {
                    FilterField::Cmc => "cmc",
                    FilterField::Power => "power",
                    FilterField::Toughness => "toughness",
                    _ => "",
                };

                const VARIABLE: &str = "*xX";

                let range = p.as_str();
                if VARIABLE.contains(range) {
                    s = match mode {
                        FilterField::Cmc => String::from("mana_cost LIKE \'%X%\'"),
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
            }
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
                                return String::from("mana_cost NOT REGEXP \'[WUBRG]+\'");
                            } else {
                                return String::from("mana_cost REGEXP \'[WUBRG]+\'");
                            }
                        }
                        "mana_cost"
                    }
                    FilterField::Identity => {
                        if a == "c" {
                            if req == ">" {
                                return String::from("color_identity = \'\'");
                            } else {
                                return String::from("color_identity != \'\'");
                            }
                        }
                        "color_identity"
                    }
                    _ => "",
                };

                s = format!("instr({}, \'{}\') {} 0", field, a.to_uppercase(), req);
            }
            Rule::rarity_val => {
                let mut a = p.as_str();
                let req = if a.starts_with('!') {
                    a = a.trim_start_matches('!');
                    "!="
                } else {
                    "="
                };

                let val = match a {
                    "c" => "common",
                    "u" => "uncommon",
                    "r" => "rare",
                    "m" => "mythic",
                    _ => "",
                };

                s = format!("rarity {req} '{val}'");
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
            let regexp: Arc<Regex> = ctx.get_or_create_aux(0, |vr| -> Result<_, BoxError> {
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

fn parse_args(column: &str, mode: ParseMode, items: &str) -> String {
    let mut v_or_conditions = Vec::new();

    let p = match &mode {
        ParseMode::Text => "{:col} {:req} \"%{:item}%\"",
        ParseMode::Tags => r#"{:col} {:req} '\|?{:item}(?:$|\|)'"#,
        ParseMode::Color => "instr({:col}, '{:item}') {:req} 0",
    };

    let groups = items.split('|');
    for mut group in groups {
        let mut v_and_conditions = Vec::new();
        let negation = group.get(0..1);
        let mut _g = String::new();
        let (req, items): (&str, Vec<&str>) = match &mode {
            ParseMode::Text => match negation {
                Some("!") => {
                    group = group.get(1..).unwrap();
                    ("NOT LIKE", group.split('&').collect())
                }
                Some(_) => ("LIKE", group.split('&').collect()),
                None => {
                    continue;
                }
            },
            ParseMode::Tags => {
                if group == "!" {
                    v_or_conditions.push(r#"tags IS NULL"#.to_string());
                    continue;
                }
                v_and_conditions.push(r#"tags IS NOT NULL"#.to_string());
                match negation {
                    Some("!") => {
                        group = group.get(1..).unwrap();
                        ("NOT REGEXP", group.split('&').collect())
                    }
                    Some(_) => ("REGEXP", group.split('&').collect()),
                    None => {
                        continue;
                    }
                }
            }
            ParseMode::Color => match negation {
                Some("!") => {
                    _g = group.get(1..).unwrap().to_uppercase();
                    ("=", _g.split_inclusive(|_c| true).collect())
                }
                Some(_) => {
                    _g = group.to_uppercase();
                    (">", _g.split_inclusive(|_c| true).collect())
                }
                None => {
                    continue;
                }
            },
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
        )",
        [],
    )?;

    conn.execute(
        "create table if not exists sets (
            id integer primary key,
            code text not null unique, 
            name text not null unique,
            date text not null,
            set_type text NOT NULL
        )",
        [],
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
        )",
        [],
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
            foreign key (commander2) references cards(name))",
        [],
    )?;

    conn.execute(
        "create table if not exists deck_contents2 (
            id integer primary key,
            card_name text not null,
            deck integer not null,
            tags text,
            foreign key (deck) references decks(id) ON DELETE CASCADE,
            unique (deck, card_name) on conflict ignore)",
        [],
    )?;

    Ok(())
}

pub fn updatedb(conn: &Connection, mut sets: Vec<Set>) -> Result<usize> {
    let mut stmt = conn.prepare("PRAGMA table_info(sets);")?;
    let mut new_cards = 0;
    sets.sort_by(|a, b| a.date.cmp(&b.date));
    let mut cols = Vec::new();
    match stmt.query([]) {
        Ok(mut results) => {
            while let Some(row) = results.next()? {
                cols.push(row.get::<usize, String>(1)?);
            }
        }
        Err(e) => {
            println!("Error!\n{:?}", e);
        }
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
    let rows: Result<Vec<Set>> = stmt
        .query_map([], |row| {
            Ok(Set {
                code: row.get(1)?,
                name: row.get(2)?,
                date: row.get(3)?,
                set_type: row.get(4)?,
            })
        })?
        .collect();
    let existing_sets = rows?;
    let mut set_stmt = conn.prepare(
        "INSERT INTO sets (code, name, date, set_type) VALUES (:code, :name, :date, :set_type);",
    )?;

    let now = Utc::now();
    let date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());

    for set in sets {
        if !existing_sets.contains(&set) && set.date <= date {
            println!("New set found: {}. Adding to existing sets.", set.name);

            let vjc = rvjc(&set.code).unwrap();
            let (success, _failure) = ivcfjsmap(conn, vjc)?;
            // println!("Added {} cards, with {} not added.", success, failure);

            set_stmt.execute(named_params! {
                ":code": set.code,
                ":name": set.name,
                ":date": set.date,
                ":set_type": set.set_type
            })?;
            new_cards += success;
        }
    }

    Ok(new_cards)
}

pub fn ucfsqlite(conn_primary: &Connection, conn_secondary: &Connection) -> Result<()> {
    let mut stmt = conn_primary.prepare("UPDATE cards SET rarity = :rarity WHERE name = :name;")?;

    let mut stmt_second = conn_secondary.prepare("SELECT name, faceName, rarity FROM cards;")?;
    let mut result = HashMap::new();
    let _a: Result<Vec<()>> = stmt_second
        .query_map([], |row| {
            let name = match row.get::<usize, String>(1) {
                Ok(s) => s,
                _ => row.get::<usize, String>(0)?,
            };
            let rarity = row.get::<usize, String>(2)?;
            if rarity != *"special" && rarity != *"bonus" {
                result.insert(name, rarity);
            }
            Ok(())
        })?
        .collect();

    let (mut success, mut failure) = (0, 0);

    println!("Generated card array. {} total cards.", result.len());
    conn_primary.execute_batch("BEGIN TRANSACTION;")?;

    for (name, rarity) in result {
        match stmt.execute(named_params! {
            ":name": name,
            ":rarity": rarity,
        }) {
            Ok(_) => {
                success += 1;
            }
            Err(_) => {
                failure += 1;
            }
        }
    }

    conn_primary.execute_batch("COMMIT TRANSACTION;")?;

    println!(
        "{} cards changed successfully. {} failures.",
        success, failure
    );

    Ok(())
}

pub fn ivcfjsmap(conn: &Connection, vjc: Vec<JsonCard>) -> Result<(usize, usize)> {
    let mut stmt = conn.prepare("INSERT INTO cards (
        name, mana_cost, cmc, types, card_text, power, toughness, loyalty, color_identity, related_cards, layout, side, legalities, rarity
    ) VALUES (
            :name, :mana_cost, :cmc, :types, :card_text, :power, :toughness, :loyalty, :color_identity, :related_cards, :layout, :side, :legalities, :rarity
    )")?;
    let (mut success, mut failure) = (0, 0);
	let mut melds = Vec::new();
	let mut meld_bases = Vec::new();
	

    print!("Found {} cards. ", vjc.len());
    conn.execute_batch("BEGIN TRANSACTION;")?;

    for c in vjc {
        let mut name = c.name.clone();
        let mut side = String::new();
        let mut related = String::new();
        match c.layout.as_str() {
            "split" | "transform" | "aftermath" | "flip" | "adventure" | "modal_dfc" => {
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
                    side = "a".to_string();
					meld_bases.push([names.1.to_string(), names.0.to_string()])
                } else {
                    related = "UNKNOWN".to_string();
                    side = "b".to_string();
					melds.push(name.clone());
                }
            }
            _ => {}
        }

        let c = c.clone();

        match stmt.execute(named_params! {
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
            Ok(_) => {
                success += 1;
            }
            Err(_) => {
                // Usually an unset or duplicate cards
                failure += 1;
            }
        }
    }

    let illegal = conn.execute(
        "DELETE
FROM cards
WHERE legalities = \"\"",
        [],
    )?;

    println!(
        "Added {} to database. {} were not added (duplicate or illegal).",
        success,
        failure + illegal
    );

    conn.execute_batch("COMMIT TRANSACTION;")?;

	for cn in &melds {
		let mut a = String::new();
		let mut b = String::new();
		for vs in &meld_bases {
			if &vs[0] == cn {
				if a == String::new() {
					a = vs[1].clone();
				} else {
					b = vs[1].clone();
				}
			}
		}
		let _ = ucfm(conn, &a, &b, cn);
		_ = ucfm(conn, &b, &a, cn);
		if a < b {
			_ = ucfm(conn, &cn, &a, &b);
		} else {
			_ = ucfm(conn, &cn, &b, &a);
		};
	}

    Ok((success, failure))
}

fn ucfm(conn: &Connection, cn: &String, ran: &String, rbn: &String) -> Result<()> {
	let rs = format!("{}|{}", ran, rbn);
	conn.execute(
		"UPDATE cards 
		SET related_cards = :related
		WHERE name = :name;",
		named_params! {":related": &rs, ":name": cn})?;

	Ok(())
}

pub fn ictodc(conn: &Connection, c: &Card, did: i32) -> Result<Vec<Card>> {
    let mut r = Vec::new();
    let mut stmt =
        conn.prepare("INSERT INTO deck_contents (card_name, deck) VALUES (:card_name, :deck_id)")?;
    stmt.execute(named_params! {":card_name": c.name, ":deck_id": did as u32})?;
    r.push(rcfn(conn, &c.name, None).unwrap());

    match &c.lo {
        CardLayout::Paired(_, _, n) => {
            stmt.execute(named_params! {":card_name": n, ":deck_id": did as u32})?;
            r.push(rcfn(conn, n, None).unwrap());
        }
        CardLayout::Meld(s, n, m) => {
            if s == &'b' {
                stmt.execute(named_params! {":card_name": n, ":deck_id": did as u32})?;
                r.push(rcfn(conn, n, None).unwrap());
                stmt.execute(named_params! {":card_name": m, ":deck_id": did as u32})?;
                r.push(rcfn(conn, m, None).unwrap());
            } else if rvcfdid(conn, did, SortOrder::NameAsc)
                .unwrap()
                .iter()
                .map(|c| c.to_string())
                .any(|x| &x == n)
            {
                stmt.execute(named_params! {":card_name": m, ":deck_id": did as u32})?;
                r.push(rcfn(conn, m, None).unwrap());
            }
        }
        _ => {}
    }

    Ok(r)
}

pub fn dcntodc(conn: &Connection, c: &str, did: i32) -> Result<()> {
    let mut stmt =
        conn.prepare("DELETE FROM deck_contents WHERE card_name = :card_name AND deck = :deck_id")?;
    stmt.execute(named_params! {":card_name": c, ":deck_id": did as u32})?;
    Ok(())
}

pub fn ttindc(conn: &Connection, c: &str, t: &String, did: i32) -> Option<Card> {
    let mut stmt = conn
        .prepare(
            "UPDATE deck_contents 
        SET tags = :tags
        WHERE card_name = :name
        AND deck = :did;",
        )
        .unwrap();

    let mut card = rcfn(conn, c, Some(did)).unwrap();
    let cc = if t.eq(&String::from("main")) || t.eq(&String::from("main")) {
        match &card.lo {
            CardLayout::Paired(_, _, n) => Some(rcfn(conn, n, Some(did)).unwrap()),
            _ => None,
        }
    } else {
        None
    };

    let tags = if card.tags.contains(t) {
        card.tags
            .remove(card.tags.iter().position(|x| x == t).unwrap());
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

    stmt.execute(named_params! {":tags": tags, ":name": c, ":did": did})
        .unwrap();
    if let Some(mut cc) = cc {
        if !card.tags.contains(t) && cc.tags.contains(t) {
            cc.tags.remove(cc.tags.iter().position(|x| x == t).unwrap());
        } else if card.tags.contains(t) && !cc.tags.contains(t) {
            cc.tags.push(t.clone());
        }
        let t = if cc.tags.is_empty() {
            None
        } else {
            Some(cc.tags.join("|"))
        };
        stmt.execute(named_params! {":tags": t, ":name": cc.name, ":did": did})
            .unwrap();
    }
    Some(card)
}

pub fn utindc(conn: &Connection, change: TagChange, cf: &CardFilter) {
    let mut stmt = conn
        .prepare(
            "UPDATE deck_contents 
        SET tags = :tags
        WHERE card_name = :name
        AND deck = :did;",
        )
        .unwrap();
    match change {
        TagChange::Delete(tag) => {
            if conn.execute_batch("BEGIN TRANSACTION;").is_err() {
                panic!("Issue with update");
            }
            let query = cf.make_query(false, format!("tag:{tag}").as_str());
            let vc = rvcfcf(conn, &query).unwrap();
            for mut c in vc {
                let i = c.tags.iter().position(|s| s == &tag).unwrap();
                c.tags.remove(i);
                let tags = if c.tags.is_empty() {
                    None
                } else {
                    Some(c.tags.join("|"))
                };
                stmt.execute(named_params! {":tags": tags, ":name": c.name, ":did": cf.did})
                    .unwrap();
            }
            if conn.execute_batch("COMMIT TRANSACTION;").is_err() {
                panic!("Issue with update");
            }
        }
        TagChange::Change(old, new) => {
            if conn.execute_batch("BEGIN TRANSACTION;").is_err() {
                panic!("Issue with update");
            }
            let query = cf.make_query(false, format!("tag:{old}").as_str());
            let vc = rvcfcf(conn, &query).unwrap();
            for mut c in vc {
                let i = c.tags.iter().position(|s| s == &old).unwrap();
                c.tags.remove(i);
                c.tags.push(new.clone());
                c.tags.sort();
                let tags = c.tags.join("|");
                stmt.execute(named_params! {":tags": tags, ":name": c.name, ":did": cf.did})
                    .unwrap();
            }
            if conn.execute_batch("COMMIT TRANSACTION;").is_err() {
                panic!("Issue with update");
            }
        }
        TagChange::Insert(_) => {}
    }
}

pub fn cindid(conn: &Connection, c: &str, did: i32) -> bool {
    let a = conn.query_row(
        "SELECT card_name FROM deck_contents WHERE card_name = ? AND deck = ?;",
        params![c, did],
        |_| Ok(0),
    );

    a.is_ok()
}

pub fn ideck(conn: &Connection, n: &str, c: &str, c2: Option<String>, t: &str) -> Result<i32> {
    match c2 {
        Some(c2) => {
            let mut stmt = conn.prepare(
                "INSERT INTO decks (name, commander, commander2, deck_type) VALUES (:name, :commander, :commander2, :deck_type);").unwrap();
            stmt.execute(
                named_params! {":name": n, ":commander": c, ":commander2": c2, ":deck_type": t},
            )
            .unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, c, None).unwrap();
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
            stmt.execute(named_params! {":name": n, ":commander": c, ":deck_type": t})
                .unwrap();
            let rid = conn.last_insert_rowid();
            let com = rcfn(conn, c, None).unwrap();
            ictodc(conn, &com, rid.try_into().unwrap()).unwrap();
            ttindc(conn, c, &String::from("main"), rid.try_into().unwrap());

            Ok(rid.try_into().unwrap())
        }
    }
}

pub fn import_deck(
    conn: &Connection,
    deck_name: String,
    coms: Vec<String>,
    cards: Vec<ImportCard>,
) -> Result<i32> {
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
                            (
                                &cards.first().unwrap().name,
                                Some(cards.get(1).unwrap().name.clone()),
                            )
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
                    let partner = ImportCard {
                        name: ss.clone(),
                        tags: None,
                    };
                    if cards.contains(&partner) {
                        println!("Valid commanders found: {} and {}", c.name, &ss);
                        (&cards.first().unwrap().name, Some(ss))
                    } else {
                        println!("Valid commander found: {}", c.name);
                        println!(
                            "But did you forget to put {} in the deck? It wasn't found.",
                            &ss
                        );
                        (&cards.first().unwrap().name, None)
                    }
                }
                CommanderType::Invalid => {
                    println!("No valid commander found! Please ensure you pass in your commander name or include it at the top of the import file.");
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
            }
        }
        1 => {
            println!("Valid commander found: {}", coms.first().unwrap());
            (coms.first().unwrap(), None)
        }
        _ => (coms.first().unwrap(), None),
    };

    if let Ok(deck_id) = ideck(conn, &deck_name, primary, secondary, "Commander") {
        println!("Deck created successfully! Now adding cards...");
        conn.execute_batch("BEGIN TRANSACTION;")?;
        let deck = rdfdid(conn, deck_id).unwrap();
        for ic in cards {
            let c = ic.name.trim().to_string();
            if c.is_empty() {
                continue;
            }
            let card = if let Some(i) = c.find(" // ") {
                let c = c.get(0..i).unwrap();
                rcfn(conn, c, None).unwrap()
            } else {
                match rcfn(conn, &c, None) {
                    Ok(a) => a,
                    Err(_) => {
                        println!("Error on card {}", c);
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                }
            };
            let mut disq = "";
            for c in &card.color_identity {
                if *c != '\u{0}' && !deck.color.contains(*c) {
                    disq = "Invalid color identity";
                }
            }
            if disq.is_empty() {
                ictodc(conn, &card, deck_id)?;
                if let Some(tags) = ic.tags {
                    for tag in tags.split('|') {
                        ttindc(conn, &card.name, &tag.to_string(), deck_id);
                    }
                };
                num += 1;
            } else {
                println!("Card not added: \"{}\" due to: {}", &card.name, disq);
            }
        }
        conn.execute_batch("COMMIT TRANSACTION;")?;
        println!("Added {} cards to deck {}", num, deck_name);
        return Ok(deck_id);
    };

    Err(rusqlite::Error::InvalidQuery)
}

pub fn rcfn(conn: &Connection, name: &str, odid: Option<i32>) -> Result<Card> {
    let mut stmt = conn.prepare("SELECT 
        cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags, rarity, price, date_price_retrieved
        FROM cards 
        LEFT OUTER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        AND deck_contents.deck = :did
        WHERE name = :name;")?;
    let mut c = stmt.query_row(named_params! {":name": name, ":did": odid}, cfr)?;

    if let CardLayout::Paired('b', _, ref rel) = c.lo {
        let side_a = rcfn(conn, &rel, None)?;
        c.price = side_a.price;
    }

    Ok(c)
}

pub fn rcomfdid(conn: &Connection, did: i32, secondary: bool) -> Result<Card> {
    let name: String = if secondary {
        conn.query_row(
            "SELECT commander2 FROM decks WHERE id = ?;",
            params![did],
            |row| match row.get::<usize, _>(0) {
                Ok(i) => Ok(i),
                Err(a) => Err(a),
            },
        )?
    } else {
        conn.query_row(
            "SELECT commander FROM decks WHERE id = ?;",
            params![did],
            |row| row.get(0),
        )?
    };

    rcfn(conn, &name, Some(did))
}

pub fn rvd(conn: &Connection) -> Result<Vec<Deck>> {
    let mut stmt = conn.prepare("SELECT * FROM decks;")?;

    let a = stmt.query_map([], |row| {
        let mut color = String::new();
        let cn: String = row.get(2)?;
        let com = rcfn(conn, &cn, None)?;
        let mut com2_colors = Vec::new();
        let com2 = match row.get::<usize, String>(3) {
            Ok(com) => {
                let b = rcfn(conn, &com, None)?;
                com2_colors = b.color_identity.clone();
                Some(b)
            }
            Err(_) => None,
        };

        for c in "WUBRG".chars() {
            if com.color_identity.contains(&c) | com2_colors.contains(&c) {
                color.push(c);
            }
        }

        if color.is_empty() {
            color = String::from("C");
        }

        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: com,
            commander2: com2,
            color,
        })
    })?;
    a.collect()
}

pub fn rdfdid(conn: &Connection, id: i32) -> Result<Deck> {
    let mut stmt = conn.prepare("SELECT * FROM decks WHERE id = ?;")?;

    stmt.query_row(params![id], |row| {
        let mut color = String::new();
        let cn: String = row.get(2)?;
        let com = rcfn(conn, &cn, None)?;
        let mut com2_colors = Vec::new();
        let com2 = match row.get::<usize, String>(3) {
            Ok(com) => {
                let b = rcfn(conn, &com, None)?;
                com2_colors = b.color_identity.clone();
                Some(b)
            }
            Err(_) => None,
        };

        for c in "WUBRG".chars() {
            if com.color_identity.contains(&c) | com2_colors.contains(&c) {
                color.push(c);
            }
        }

        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
            commander: com,
            commander2: com2,
            color,
        })
    })
}

pub fn rvcfdid(conn: &Connection, did: i32, sort_order: SortOrder) -> Result<Vec<Card>> {
    let (order, sort_on) = match sort_order {
        SortOrder::NameAsc => (String::from("ASC"), String::from("name")),
        SortOrder::NameDesc => (String::from("DESC"), String::from("name")),
        SortOrder::CmcAsc => (String::from("ASC"), String::from("cmc")),
        SortOrder::CmcDesc => (String::from("DESC"), String::from("cmc")),
        SortOrder::PriceAsc => (String::from("ASC"), String::from("price")),
        SortOrder::PriceDesc => (String::from("DESC"), String::from("price")),
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

    let a = stmt.query_map(named_params! { ":did": did }, cfr)?;
    a.collect()
}

pub fn rvcfcf(conn: &Connection, query: &str) -> Result<Vec<Card>> {
    let fields = "cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side, tags, rarity, price, date_price_retrieved";
    let qs = format!(
        "SELECT {}
FROM `cards`
{}",
        fields, query
    );

    let mut stmt = conn.prepare(&qs).expect("issue with filter string");

    let cards = stmt.query_map([], cfr)?.collect();

    cards
}

pub fn rvcnfcf(conn: &Connection, query: &str) -> Result<Vec<String>> {
    let fields = "name";
    let qs = format!(
        "SELECT {}
FROM cards
{}",
        fields, query
    );
    let mut stmt = conn.prepare(&qs).expect("issue with filter string");

    let cards = stmt.query_map([], |row| row.get(0))?.collect();

    cards
}

pub fn rvcnfn(conn: &Connection, n: &str) -> Result<Vec<String>> {
    if n.is_empty() {
        return Ok(Vec::new());
    }
    let query = format!(
        "
SELECT name
FROM cards
WHERE name LIKE \"%{}%\"
AND types LIKE \'Legendary%\'
AND (types LIKE \'%Creature%\' OR card_text LIKE \'%can be your commander%\')
ORDER BY name ASC;",
        n
    );
    let mut stmt = conn.prepare(query.as_str())?;

    let a = stmt.query_map([], |row| row.get(0))?;
    a.collect()
}

pub fn rvcnfnp(conn: &Connection, n: &str) -> Result<Vec<String>> {
    if n.is_empty() {
        return Ok(Vec::new());
    }
    let query = format!(
        "
SELECT name
FROM cards
WHERE name LIKE \"%{}%\"
AND types LIKE 'Legendary%'
AND card_text LIKE '%Partner%'
AND (types LIKE '%Creature%' OR card_text LIKE '%can be your commander%')
ORDER BY name ASC;",
        n
    );
    let mut stmt = conn.prepare(query.as_str())?;

    let a = stmt.query_map([], |row| row.get(0))?;
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

    for ch in sch.split('|') {
        vch.push(ch.chars().next().unwrap_or_default());
    }

    vch
}

fn cfr(row: &Row) -> Result<Card> {
    let lo = match row.get::<usize, String>(10) {
        Ok(s) => match s.as_str() {
            "adventure" => {
                let rel = row.get::<usize, String>(11)?;
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                if side == 'a' {
                    CardLayout::Paired(side, String::from("Also has Adventure"), rel)
                } else {
                    CardLayout::Paired(side, String::from("Adventure of"), rel)
                }
            }
            "aftermath" => {
                let rel = row.get::<usize, String>(11)?;
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                if side == 'a' {
                    CardLayout::Paired(side, String::from("Also has Aftermath"), rel)
                } else {
                    CardLayout::Paired(side, String::from("Aftermath of"), rel)
                }
            }
            "flip" => {
                let rel = row.get::<usize, String>(11)?;
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                if side == 'a' {
                    CardLayout::Paired(side, String::from("Also has Flip side"), rel)
                } else {
                    CardLayout::Paired(side, String::from("Flip side of"), rel)
                }
            }
            "leveler" => CardLayout::Leveler,
            "meld" => {
                let rel = row.get::<usize, String>(11)?;
                let rels = rel.split_once('|').unwrap();
                let (face, transform) = (String::from(rels.0), String::from(rels.1));
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                CardLayout::Meld(side, face, transform)
            }
            "modal_dfc" => {
                let rel = row.get::<usize, String>(11)?;
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                if side == 'a' {
                    CardLayout::Paired(side, String::from("Modal Face. You may instead cast"), rel)
                } else {
                    CardLayout::Paired(side, String::from("Modal Back. You may instead cast"), rel)
                }
            }
            "normal" => CardLayout::Normal,
            "saga" => CardLayout::Saga,
            "split" => {
                let rel = row.get::<usize, String>(11)?;
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                CardLayout::Paired(side, String::from("You may instead cast"), rel)
            }
            "transform" => {
                let rel = row.get::<usize, String>(11)?;
                let side = row.get::<usize, String>(12)?.chars().next().unwrap();
                if side == 'a' {
                    CardLayout::Paired(side, String::from("Transforms into"), rel)
                } else {
                    CardLayout::Paired(side, String::from("Transforms from"), rel)
                }
            }
            _ => CardLayout::Normal,
        },
        Err(_) => CardLayout::Normal,
    };

    let tags: Vec<String> = match row.get::<usize, String>(13) {
        Ok(a) => stovs(a),
        Err(_) => Vec::new(),
    };

    let price = match row.get(15) {
        Ok(a) => Some(a),
        Err(_) => None,
    };

    let stale = match row.get::<usize, String>(16) {
        Ok(a) => {
            let date = Utc::today();
            let vs: Vec<u32> = a.split('-').map(|s| s.parse::<u32>().unwrap()).collect();
            let ret = Utc.ymd(vs[0] as i32, vs[1], vs[2]);

            date - ret > Duration::days(14)
        }
        Err(_) => true,
    };

    Ok(Card {
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
    let mut stmt = conn
        .prepare(
            r#"SELECT
        card_name, tags
        FROM deck_contents
        WHERE deck = :did;"#,
        )
        .unwrap();

    let r = stmt
        .query_map(named_params! {":did": did}, |row| {
            Ok(ImportCard {
                name: row.get(0)?,
                tags: row.get(1)?,
            })
        })?
        .collect();

    r
}

pub fn rvmcfd(conn: &Connection, did: i32) -> Result<Vec<CardStat>> {
    let mut stmt = conn
        .prepare(
            r#"SELECT
        cmc, color_identity, mana_cost, name, tags, types, price, date_price_retrieved
        FROM cards
        INNER JOIN deck_contents
        ON cards.name = deck_contents.card_name
        WHERE deck_contents.deck = :did
        AND (side != 'b' OR layout == 'split' OR layout == 'modal_dfc')
        AND tags IS NOT NULL 
        AND tags REGEXP '\|?main(?:$|\|)';"#,
        )
        .unwrap();
    let date = Utc::today();

    let a = stmt
        .query_map(named_params! {":did": did}, |row| {
            let stale = match row.get::<usize, String>(7) {
                Ok(a) => {
                    let vs: Vec<u32> = a.split('-').map(|s| s.parse::<u32>().unwrap()).collect();
                    let ret = Utc.ymd(vs[0] as i32, vs[1], vs[2]);

                    date - ret > Duration::days(14)
                }
                Err(_) => true,
            };
            Ok(CardStat {
                cmc: row.get::<usize, f64>(0)? as u8,
                color_identity: stovs(row.get(1)?),
                mana_cost: row.get(2)?,
                name: row.get(3)?,
                tags: stovs(row.get(4)?),
                types: row.get(5)?,
                price: if let Ok(i) = row.get(6) { i } else { 0.0 },
                stale,
            })
        })?
        .collect();
    a
}

pub fn ucfd(rwl_conn: &Mutex<Connection>, did: i32) -> Result<()> {
    let unpriced: Result<Vec<(String, String, String)>> = {
        let conn = rwl_conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT name, layout, related_cards
                FROM cards
                INNER JOIN deck_contents
                ON cards.name = deck_contents.card_name
                WHERE deck_contents.deck = :did
                AND side != 'b'
                AND (date_price_retrieved ISNULL OR date_price_retrieved < date('now','-6 day'))
                AND tags IS NOT NULL 
                AND tags REGEXP '\|?main(?:$|\|)';"#,
            )
            .unwrap();
        let a = stmt
            .query_map(named_params! {":did": did}, |row| {
                Ok((
                    row.get::<usize, String>(0)?,
                    row.get::<usize, String>(1)?,
                    row.get::<usize, String>(2)?,
                ))
            })?
            .collect();

        a
    };

    let delay = time::Duration::from_millis(50);
    for (name, layout, related) in unpriced.unwrap() {
        thread::sleep(delay);
        let price = rpfdc(&name, &layout, &related).unwrap();
        rwl_conn.lock().unwrap().execute(
            "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
            named_params! {":price": price, ":name": name},
        )?;
    }

    Ok(())
}

pub fn upfcn_quick(conn: &Connection, cn: &str, odid: Option<i32>) -> Result<Card> {
    let c = rcfn(conn, cn, None).unwrap();
    let s = match &c.lo {
        CardLayout::Paired(_, _, rel) => format!("{} // {}", cn, rel),
        _ => cn.to_string(),
    };

    let price = rcostfcn(&s, c.price).unwrap();

    match &c.lo {
        CardLayout::Paired(_, n, rel) =>{
            conn.execute(
                "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
                named_params! {":price": price, ":name": &n},
            )?;
            conn.execute(
                "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
                named_params! {":price": price, ":name": &rel},
            )?;
        },
        _ => {
            conn.execute(
                "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
                named_params! {":price": price, ":name": &c.name},
            )?;
        }
    };

    rcfn(conn, cn, odid)
}

pub fn upfcn_detailed(conn: &Connection, c: &Card, odid: Option<i32>) -> Result<Card> {
    let cn = match &c.lo {
        CardLayout::Paired('a', _, rel) => format!("{} // {}", c.name, rel),
        CardLayout::Paired('b', _, rel) => format!("{} // {}", rel, c.name),
        _ => c.name.clone(),
    };

    let price = rextcostfcn(&cn).unwrap();

    match &c.lo {
        CardLayout::Paired(_, n, rel) =>{
            conn.execute(
                "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
                named_params! {":price": price, ":name": &n},
            )?;
            conn.execute(
                "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
                named_params! {":price": price, ":name": &rel},
            )?;
        },
        _ => {
            conn.execute(
                "UPDATE cards 
                SET price = :price, 
                date_price_retrieved = date()
                WHERE name = :name;",
                named_params! {":price": price, ":name": &c.name},
            )?;
        }
    };

    rcfn(conn, &c.name, odid)
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
    conn.execute(
        "DELETE FROM decks WHERE id = :did",
        named_params! {":did": did},
    )?;
    Ok(())
}

pub fn rpfdc(name: &str, layout: &str, related: &str) -> Result<f64> {
    let s = if !related.is_empty() && layout != "meld" {
        format!("{} // {}", name, related)
    } else {
        name.to_string()
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
    pub layout: String,
    // pub related_cards: Option<Relation>,
    pub side: Option<char>,
    pub rarity: String,
}

fn zero() -> String {
    String::from("0")
}

impl JsonCard {
    pub fn convert(&self) -> Card {
        todo!();
    }
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
            serde_json::Value::Object(i) => i,
            _ => return Vec::new(),
        };

        legalities.keys().cloned().collect()
    }

    fn from(s: String) -> Legalities {
        let mut l = Legalities::default();

        if s.contains("commander") {
            l.commander = String::from("Allowed");
        }

        l
    }
}

impl ToString for Legalities {
    fn to_string(&self) -> String {
        let mut vs = Vec::new();
        let b = vec![String::default(), String::from("Banned")];

        if !b.contains(&self.brawl) {
            vs.push("brawl");
        }
        if !b.contains(&self.commander) {
            vs.push("commander");
        }
        if !b.contains(&self.modern) {
            vs.push("modern");
        }
        if !b.contains(&self.standard) {
            vs.push("standard");
        }

        vs.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::util::get_local_file;
    use std::env::current_dir;

    #[test]
    fn complex_query() {
        let p = current_dir().unwrap().join("target/debug/lieutenant.db");
        let conn = Connection::open(p).unwrap();
        add_regexp_function(&conn).unwrap();
        let decks = rvd(&conn).unwrap();
        let a = decks.iter().find(|d| d.name == "test" && d.commander.name == "Mizzix of the Izmagnus");
        assert!(a.is_some());

        if let Some(deck) = a {

            let mut cf = CardFilter::default();
            cf.did = deck.id;
            let query = cf.make_query(false, "ty:shaman");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 5);
            let query = cf.make_query(false, "ty:shaman te:sacrifice");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 4);
            let query = cf.make_query(false, "ty:shaman te:sacrifice r:c");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 3);
            let query = cf.make_query(false, "ty:shaman te:sacrifice r:c tag:main");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 2);
            let query = cf.make_query(false, "ty:shaman te:sacrifice r:c tag:main cmc:1");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 1);
            assert_eq!(res.first().unwrap().name, String::from("Krark-Clan Shaman"));
        }
    }

    #[test]
    fn apostrophes() {
        let p = current_dir().unwrap().join("target/debug/lieutenant.db");
        let conn = Connection::open(p).unwrap();
        add_regexp_function(&conn).unwrap();
        let decks = rvd(&conn).unwrap();
        let a = decks.iter().find(|d| d.name == "test" && d.commander.name == "Mizzix of the Izmagnus");
        assert!(a.is_some());

        if let Some(deck) = a {
            let mut cf = CardFilter::default();
            cf.did = deck.id;
            let query = cf.make_query(false, "na:\"'\"");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 1);
            assert_eq!(res.first().unwrap().name, String::from("Blue Sun's Zenith"));
            let query = cf.make_query(false, "te:\"'\"");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 1);
            assert_eq!(res.first().unwrap().name, String::from("Blue Sun's Zenith"));
        }
    }

    #[test]
    fn x_mana_cost() {
        let p = current_dir().unwrap().join("target/debug/lieutenant.db");
        let conn = Connection::open(p).unwrap();
        add_regexp_function(&conn).unwrap();
        let decks = rvd(&conn).unwrap();
        let a = decks.iter().find(|d| d.name == "test" && d.commander.name == "Mizzix of the Izmagnus");
        assert!(a.is_some());

        if let Some(deck) = a {
            let mut cf = CardFilter::default();
            cf.did = deck.id;
            let query = cf.make_query(false, "cmc:*");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 1);
            assert_eq!(res.first().unwrap().name, String::from("Blue Sun's Zenith"));
        }
    }

    #[test]
    fn colors() {
        let p = current_dir().unwrap().join("target/debug/lieutenant.db");
        let conn = Connection::open(p).unwrap();
        add_regexp_function(&conn).unwrap();
        let decks = rvd(&conn).unwrap();
        let a = decks.iter().find(|d| d.name == "test" && d.commander.name == "Mizzix of the Izmagnus");
        assert!(a.is_some());

        if let Some(deck) = a {
            let mut cf = CardFilter::default();
            cf.did = deck.id;
            let query = cf.make_query(false, "c:c");
            let res = rvcfcf(&conn, &query).unwrap();
            assert_eq!(res.len(), 2);
            let query = cf.make_query(false, "ci:c");
            let res = rvcfcf(&conn, &query).unwrap();
            assert!(res.is_empty());
        }
    }
}