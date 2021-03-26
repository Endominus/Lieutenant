Idea; Add a notes column to the decks.

https://mtgjson.com
https://scryfall.com/docs/api/
https://docs.magicthegathering.io/

# 1.0 Feature List
- Remember most recently opened deck and allow direct loading.
- Add, edit, and remove decks.
  - Implement Create Deck screen.
  - Takes Deck Name, Commander, and optionally a file to import.
  - Import needs to handle the '//' case of split cards, and automatically adding the other half of cards.
  - Modify deck table to include basic lands somehow.
- Manipulate cards in a deck.
  - Add and remove cards. Automatically add the other half of split or transform cards.
  - Add and remove tags from cards.
  - Jump from a card to its related card.
- Browse and filter cards in database.
  - Filter by name, text, type, subtype, cmc, color, color identity, power, toughness, and tag.
  - Sort by name and cmc, ascending and descending
- View deck info.
  - Include mana curve, color groups, type breakdowns
  - Determine the number of "real" cards in deck and warn if that is too high.

1.1 Features
- In DbView, highlight cards already in deck.
- Add setting to automatically load into most recent deck.
- Get deck pricing. (see Scryfall?). Include as property of Deck struct with date retrieved.

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

"Abandon Hope": [{
  "colorIdentity": ["B"], 
  "colors": ["B"], 
  "convertedManaCost": 2.0, 
  "edhrecRank": 11487, 
  "foreignData": [{"language": "German", "name": "Verlust der Hoffnung", "text": "Wähle X Karten aus Deiner Hand und wirf diese ab: Schau Dir die Hand eines Gegners Deiner Wahl an und wähle X Karten aus, die er abwerfen muß.", "type": "Hexerei"}, {"language": "Spanish", "name": "Perder la esperanza"}, {"language": "French", "name": "Abandon de l'espoir"}, {"language": "Italian", "name": "Abbandonare la Speranza"}, {"language": "Japanese", "name": "断念"}, {"language": "Portuguese (Brazil)", "name": "Abandonar a Esperança"}], 
  "identifiers": {"scryfallOracleId": "8adbba6e-03ef-4278-aec5-8a4496b377a8"}, 
  "layout": "normal", 
  "legalities": {
      "commander": "Legal", 
      "duel": "Legal", 
      "legacy": "Legal", 
      "penny": "Legal", 
      "premodern": "Legal", 
      "vintage": "Legal"
  }, 
  "manaCost": "{X}{1}{B}", 
  "name": "Abandon Hope", 
  "printings": ["TMP"], 
  "purchaseUrls": {"cardKingdom": "https://mtgjson.com/links/07d7e7455de8fed7", "cardmarket": "https://mtgjson.com/links/620bc66b2a5bbcf0", "tcgplayer": "https://mtgjson.com/links/10a47fee5c2a372d"}, 
  "rulings": [], 
  "subtypes": [], 
  "supertypes": [], 
  "text": "As an additional cost to cast this spell, discard X cards.\nLook at target opponent's hand and choose X cards from it. That player discards those cards.", 
  "type": "Sorcery", 
  "types": ["Sorcery"]
}],
