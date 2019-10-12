use rusqlite::{params, Connection, Result, NO_PARAMS};
use rusqlite::types::ToSql;


pub fn retrieve_card(name: String) -> Card {
    unimplemented!
}

pub fn create_db() -> Result<()> {
    let conn = Connection::open("cards.db")?;

    conn.execute(
        "create table if not exists cat_colors (
            id integer primary key,
            name text not null unique)", 
            NO_PARAMS,
    )?;

    Ok(())
}