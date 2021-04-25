Idea; Add a notes column to the decks.

https://mtgjson.com
https://scryfall.com/docs/api/
https://docs.magicthegathering.io/

# 1.0 Feature List
- Remember most recently opened deck and allow direct loading.
- Add, edit, and remove decks.
  - ~~Implement Create Deck screen.~~
  - Takes Deck Name, Commander, and optionally a file to import.
  - View list of commanders as the user types. Can go up and down to select the specific one.
  - ~~List of potential commanders must include all potential commanders, including planeswalkers and partners.~~
  - ~~Import needs to handle the '//' case of split cards, and automatically adding the other half of cards.~~
- Manipulate cards in a deck.
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
    - rarity,
    - power, 
    - toughness,
  - Sort by name and cmc, ascending and descending
  - Automatically filter by ~~deck color~~ and commander legality(?).
- View deck info.
  - Include ~~mana curve~~, color groups, ~~type breakdowns~~
  - Determine the number of "real" cards in deck and warn if that is too high or low.
  - Warn the user of any illegal cards(?)
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

ana
ana te:and ty:creature+legendary st:cleric cmc:1-3 c:wr c:wb ci
te:"draw a card"
c:wr
c:w+r
c:w|r

Enchantment
Creature
Land
Instant
Sorcery
Artifact

