extern crate rusqlite;

use self::rusqlite::{params, Connection, Result, NO_PARAMS};

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

fn ic(vc: Vec<crate::Card>) -> Result<()> {
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

pub fn rc_name(name: String) -> Vec<crate::Card> {
    unimplemented!()
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
            card_text text not null,
            mana_cost text not null,
            layout text not null,
            types text not null,
            supertypes text not null,
            subtypes text not null,
            color_identity text not null,
            related_cards text not null,
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
    let conn = Connection::open("cards.db")?;
    let mut stmt = conn.prepare("select s.code, s.name from sets s;")?;
    let si = stmt
        .query_map(NO_PARAMS, |row|
        {
            Ok(
                Set {
                    code: row.get(0)?,
                    name: row.get(1)?,
                }
            )
        })?;
    let so = crate::network::rs().unwrap();
    let mut sd = Vec::new();

    let si: Vec<Set> = si.filter_map(Result::ok).collect();

    for s in so {
        if !si.contains(&s) 
            && !BANNED.contains(&&(*s.code)) {
            sd.push(s.clone());
        }
    }

    for s in sd {
        let c = crate::network::rcs(&s);
        ic(c)?;
        println!("Inserted all cards in {}", s.name);
        is(s)?;
    }
    Ok(())
}