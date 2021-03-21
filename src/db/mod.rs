extern crate rusqlite;

use self::rusqlite::{params, Connection, Result, NO_PARAMS};
use crate::{Card, Deck};
use std::fs;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Set {
    pub code: String,
    name: String,
}

const BANNED: [&'static str; 6] = [
    "UGL", "UNH", "UST", "H17", "HHO", "HTR"
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

pub fn rvct(mut text: String, did: i32) -> Result<Vec<Card>> {
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

pub fn rvcn(mut name: String, did: i32) -> Result<Vec<Card>> {
    let conn = Connection::open("cards.db")?;

    name.insert(0, '%');
    name.push('%');
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
            name text not null unique)"
            , NO_PARAMS,
    )?;

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
            cmc integer not null)"
            , NO_PARAMS)?;

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
            , NO_PARAMS)?;
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
    
    // let si: Vec<Set> = si.filter_map(Result::ok).collect();
    // let mut sd = Vec::new();
    // for s in so {
    //     if !si.contains(&s) 
    //         && !BANNED.contains(& s.code.as_str()) {
    //         println!("Set '{}' not found in database! Preparing to download...", s.name);
    //         sd.push(s.clone());
    //     }
    // }

    for s in sd {
        println!("Found set '{}' missing. Retrieving cards now.", s.name);
        let vc = crate::network::rcs(&s);
        ivctoc(vc)?;
        println!("Inserted all cards in {}", s.name);
        is(s)?;
    }
    Ok(())
}

pub fn rvcd(did: i32) -> Result<Vec<Card>> {
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

pub fn rc(name: String) -> Result<Card> {
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
            commander: rc(row.get(2)?)?,
        })
    })?.collect();

    decks
}