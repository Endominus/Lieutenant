#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate reqwest;

mod network;
mod db;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub name: String,
    pub supertypes: Vec<String>,
    pub types: Vec<String>,
    pub subtypes: Vec<String>,
    pub text: String,
    pub cmc: f64,
    #[serde(default = "zero")]
    pub mana_cost: String,
    pub color_identity: Vec<String>,
    #[serde(rename = "names")]
    #[serde(default)]
    pub related_cards: Vec<String>,
    pub layout : String,
}

fn zero() -> String { String::from("0") }


// impl Card {
//     pub fn new(
//         name: String, 
//         types: Vec<String>, 
//         supertypes: Vec<String>,
//         subtypes: Vec<String>,
//         text: String,
//         cmc: i8,
//         mana_cost: String,
//         color_identity: Vec<String>,
//         related_cards: Vec<String>,
//         layout: String) -> Card {
//             Card {
//                 name,
//                 types,
//                 supertypes,
//                 subtypes,
//                 text,
//                 cmc,
//                 mana_cost,
//                 color_identity,
//                 related_cards,
//                 layout
//             }
//         }
// }

pub enum Command {
    RetrieveCard(String),
    FullPull,
    UpdateDB
}

pub fn run(command: Command) -> Result<(), reqwest::Error> {
    match command {
        Command::RetrieveCard(card) => {
            let a = network::retrieve_card_by_name(card)?;
            for card in a {
                println!("{:?}", card);
            }

            Ok(())
        },
        Command::FullPull => {
            // db::create_db();
            // db::full_pull();
            network::rs();
            // println!("{:?}", a);
            Ok(())
        },
        Command::UpdateDB => {unimplemented!()},
    }
}

