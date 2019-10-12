extern crate reqwest;
mod network;

#[derive(Deserialize, Debug)]
pub struct Card {
    pub name: String,
    pub types: Vec<String>,
    pub subtypes: Vec<String>,
    pub text: String,
    pub cmc: i8,
    pub mana_cost: String,
    pub color_identity: Vec<char>,
    pub related_cards: Vec<String>
}

impl Card {
    pub fn new(
        name: String, 
        types: Vec<String>, 
        subtypes: Vec<String>,
        text: String,
        cmc: i8,
        mana_cost: String,
        color_identity: Vec<char>,
        related_cards: Vec<String>) -> Card {
            Card {
                name,
                types,
                subtypes,
                text,
                cmc,
                mana_cost,
                color_identity,
                related_cards
            }
        }
}

pub enum Command {
    RetrieveCard(String),
    FullPull,
    UpdateDB
}

pub fn run(command: Command) -> Result<(), &'static str> {
    match command {
        Command::RetrieveCard(card) => {
            let a = network::retrieve_card_by_name(card)?;
            for card in a {
                println!("{:?}", card);
            }

            Ok(())
        },
        Command::FullPull => {
            network::full_pull();
            Ok(())
        },
        Command::UpdateDB => {unimplemented!()},
    }
}

