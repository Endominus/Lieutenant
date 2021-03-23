Idea; Add a notes column to the decks.

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
- View deck info.
  - Include mana curve, color groups, type breakdowns
  - Determine the number of "real" cards in deck and warn if that is too high.

1.1 Features
- In DbView, highlight cards already in deck.
- Add setting to automatically load into most recent deck.
