https://mtgjson.com
https://scryfall.com/docs/api/
https://docs.magicthegathering.io/

# 1.0 Feature List
- Remember most recently opened deck and allow direct loading.
- Add, edit, and remove decks.
  - ~~Implement Create Deck screen.~~
  - Takes Deck Name, Commander, and optionally a file to import.
  - ~~View list of commanders as the user types. Can go up and down to select the specific one.~~
  - ~~List of potential commanders must include all potential commanders, including planeswalkers and partners.~~
  - ~~Import needs to handle the '//' case of split cards, and automatically adding the other half of cards.~~
- ~~Manipulate cards in a deck.~~
  - ~~Add and remove cards.~~
  - ~~Automatically add the other half of split or transform cards.~~
  - ~~Add and remove tags from cards.~~
  - ~~Jump from a card to its related card.~~
- Browse and filter cards in database.
  - Filter by; 
    - ~~name,~~
    - ~~text,~~
    - ~~type,~~
    - ~~cmc,~~
    - ~~color,~~
    - ~~color identity,~~
    - ~~tag,~~
    - ~~rarity,~~
    - ~~power,~~ 
    - ~~toughness,~~
  - ~~Sort by name and cmc, ascending and descending~~
  - Automatically filter by ~~deck color~~ and commander legality(?).
- View deck info.
  - Include: 
    - ~~mana curve~~
    - ~~type breakdowns~~
    - ~~mana symbol amounts~~
    - ~~tag list~~
    - ~~prices~~
  - Caution the user if:
    - Number of "real" cards in deck is too high/low.
    - Any illegal cards in the deck
    - Excessively high mana curve
- Update the card database with new sets
- 

# 1.1 Feature List
- In DbView, highlight cards already in deck.
- Add setting to automatically load into most recent deck.
- ~~Get deck pricing. (see Scryfall?). Include as property of Card row with date retrieved.~~
  - Do this in the background.
- ~~Quick change tags~~
- Resize DeckStat for different form factors.
- "Basics" list for each color combo.
- Filter history, same as with a shell.

# 2.0 Feature List
- Deck notes?

tags should be applied to castable spells only (i.e. not transformed)?


           let cards = db::rvcfdid(&conn, 2, util::SortOrder::NameAsc).unwrap();
            println!("{}", cards.get(0).unwrap().name);
            let cards = db::rvcfdid(&conn, 2, util::SortOrder::NameDesc).unwrap();
            println!("{}", cards.get(0).unwrap().name);
            let cards = db::rvcfdid(&conn, 2, util::SortOrder::CmcAsc).unwrap();
            println!("{}", cards.get(0).unwrap().name);
            // let deck = db::rdfdid(&conn, 1).unwrap();
            // let omni = String::new();
            // let cf = db::CardFilter::from(&deck, &omni);
            // println!("Cardfilter produces: {}", cf.make_filter(true));
            // let a = SETTINGS;



            // let mut s = Config::default();
            // s.merge(config::File::with_name("settings.toml")).unwrap();
            // println!("{:?}", s);


    // TODO if settings file doesn't exist, create it with default values.

            // println!("{:?}", SETTINGS.read().unwrap().clone().try_into::<HashMap<String, Settings>>().unwrap());
            // println!("{:?}", SETTINGS.read().unwrap().get_tags());
            // println!("{:?}", SETTINGS.read().unwrap().get_tags_deck(1));
            // println!("{:?}", SETTINGS.read().unwrap().get_tags_deck(2));
            // println!("{:?}", SETTINGS.read().unwrap().get_tags_deck(3));
            // SETTINGS.write().unwrap().set("recent", 1).unwrap();
            // println!("{:?}", SETTINGS.read().unwrap().clone().try_into::<HashMap<String, String>>().unwrap());

            // let a = network::rcs(& db::Set { code: String::from("TPH1"), name: String::from("Theros Path of Heroes") });
            // let cf = db::CardFilter::new(1).text(String::from("ana"));
            // println!("Cardfilter produces: {}", cf.make_filter());
            // println!("{:?}", CardFilter::parse_omni("ana"));
            // println!("{:?}", CardFilter::parse_omni("n:\"kor sky\""));
            // println!("{:?}", CardFilter::parse_omni("name:\" of \""));
            // println!("{:?}", CardFilter::parse_omni("text:\"draw a card\""));
            // println!("{:?}", CardFilter::parse_omni("text:\"+1\""));
            // println!("{:?}", CardFilter::parse_omni("text:\"+1\" n:aja"));
            // println!("{:?}", CardFilter::parse_omni("text:\"+1\" n:aja ty:creature"));
            // println!("{:?}", CardFilter::parse_omni("te:lifelink"));
            // println!("{:?}", CardFilter::parse_omni("te:\"draw a card\" n:Ajani"));
            // println!("{:?}", CardFilter::parse_omni("color:c"));
            // println!("{:?}", CardFilter::parse_omni("c:w name:blue"));
            // println!("{:?}", CardFilter::parse_omni("c:wb"));
            // println!("{:?}", CardFilter::parse_omni("color:w|b"));
            // println!("{:?}", CardFilter::parse_omni("color:b|g/w"));
            // println!("{:?}", CardFilter::parse_omni("type:creature"));
            // println!("{:?}", CardFilter::parse_omni("ty:legendary+sorcery"));
            // println!("{:?}", CardFilter::parse_omni("ty:legendary+creature/sorcery+tribal/instant name:\"how are you\""));
            // println!("{:?}", CardFilter::parse_omni("ty:c"));
            // println!("{:?}", CardFilter::parse_omni("ty:coward"));
            // println!("{:?}", CardFilter::parse_omni("ty:instant te:draw ajani"));
            // println!("{:?}", CardFilter::parse_omni("cmc:0-4"));
            // println!("{:?}", CardFilter::parse_omni("cmc:-4"));
            // println!("{:?}", CardFilter::parse_omni("cmc:4-"));
            // println!("{:?}", CardFilter::parse_omni("cmc:<10"));
            // println!("{:?}", CardFilter::parse_omni("cmc:>10"));
            // println!("{:?}", CardFilter::parse_omni("ci:wb"));
            // println!("{:?}", CardFilter::parse_omni("ci:wr"));
            // println!("{:?}", CardFilter::parse_omni("coloridentity:w/b"));
            // println!("{:?}", CardFilter::parse_omni("color_identity:b|g|w"));
            // println!("{:?}", CardFilter::parse_omni("p:0-4"));
            // println!("{:?}", CardFilter::parse_omni("p:-4"));
            // println!("{:?}", CardFilter::parse_omni("power:4-"));
            // println!("{:?}", CardFilter::parse_omni("power:-"));
            // println!("{:?}", CardFilter::parse_omni("power:"));
            // println!("{:?}", CardFilter::parse_omni("n:"));
            // println!("{:?}", CardFilter::parse_omni("n:\"\""));
            // println!("{:?}", CardFilter::parse_omni("power:"));
            // println!("{:?}", CardFilter::parse_omni("te:"));
            // println!("{:?}", CardFilter::parse_omni("color:"));
            // println!("{:?}", CardFilter::parse_omni("c:"));
            // println!("{:?}", CardFilter::parse_omni("ty:"));
            // println!("{:?}", CardFilter::parse_omni("cmc:"));
            // println!("{:?}", CardFilter::parse_omni("coloridentity:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni(""));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("n:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("n:\"\""));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("power:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("te:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("color:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("c:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("ty:"));
            // // assert_eq!(HashMap::new(), CardFilter::parse_omni("cmc:"));
            // assert_eq!(HashMap::new(), CardFilter::parse_omni("coloridentity:"));
            
            // let omni = String::from("n:\"kor sky\" ty:artifact cmc:>4 te:w|");
            // let omni = String::from("ty:artifact cmc:>4 color:w|");
            // let omni = String::from("ty:artifact ci:wr cmc:2-");
            // let cf = CardFilter::from(5, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("ty: cmc:>4");
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("cmc:>4 tag:");
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("cmc:>4 color_identity:");
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // let omni = String::from("text:\"you control can\'t\" c:r|g");
            // let omni = String::from("cmc:<4 tag:ramp"); //tags REGEXP '\|?ramp(?:$|\|)'
            // let omni = String::from("cmc:<4 ci:c"); //color_identity REGEXP '^[^WUBRG]*$'
            // let cf = CardFilter::from(1, & omni);
            // println!("{}", cf.make_filter(&conn, false));
            // println!("{:?}", db::rvcfcf(&conn, cf, false));//.iter().map(|f| f.to_string()).collect::<Vec<String>>());

            // let s = "WHERE regexp('.*ozi.*', name)";
            // let s = "WHERE name REGEXP \'.*ozi.*\' AND mana_cost REGEXP \'R\'";
            // let s = "WHERE cards.name LIKE \'%ana%\'";
            // let s = "%ana%";
            // println!("{:?}", db::db_test(s).unwrap().len());

            // let now = Instant::now();
            // let file = File::open("AtomicCards.json").unwrap();
            // let reader = BufReader::new(file);
            // let a: serde_json::Value = serde_json::from_reader(reader).unwrap();
            // println!("Imported cards in {} s.", now.elapsed().as_secs());
            // let now = Instant::now();
            // let _iresult = db::initdb(&conn);
            // let (a, b) = db::ivcfjsmap(&conn, a).unwrap();
            // println!("Inserted {} rows with {} failures in {} ms.", a, b, now.elapsed().as_millis());
            // println!("{}", a["data"]["Chalice of Life // Chalice of Death"]);
            // let c: NewCard = serde_json::from_value(
            //     a["data"]["Chalice of Life // Chalice of Death"].clone())
            //     .unwrap();
            // println!("{:?}", c)
            
            // println!("{} cards in {} s", vc.len(), now.elapsed().as_secs());
            // let now = Instant::now();
            // let a = db::create_new_database();
            // println!("{:?}: Cards added to database in {} s", a, now.elapsed().as_secs());

            // let future = async move {
            //     let a = rcostfcn(&"Raging Goblin".to_string()).await;
            //     println!("{:?}", a)
            // };

            // let res = tokio::runtime::Builder::new()
            //     .basic_scheduler()
            //     .enable_all()
            //     .build()
            //     .unwrap()
            //     .block_on(future);
            // res
            // let _a = db::ucfd(&conn, 2);