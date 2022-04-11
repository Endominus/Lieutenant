# Lieutenant

Lieutenant is a tool to help people build their commander decks. It allows for much faster and easier card filtering than using any online database that I've found, and allows you to add tags to cards to better understand capabilities and weaknesses in your decks.

## "Installing"

Just download the archive file corresponding to your OS from the [latest release](https://github.com/Endominus/Lieutenant/releases/latest) and extract it. The database and settings file need to be in the same directory as the executable.

<iframe width="1280" height="720" src="https://www.youtube.com/embed/8BOfkMagso8" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## Using the Software

Before anything else, you should probably set up your default tags in the main menu's Settings page. These tags will be added by default to any subsequent deck you make; they can still be deleted on a case-by-case basis if you're not going to use them in that deck. A few have already been created as examples, such as "draw", "removal", and "board_wipe". Note that the "main" tag cannot be deleted or changed, as the software requires it to know what cards are in your maindeck.

To create a deck, it must have a name and at least one commander. Technically, I suppose you could leave the name field blank, but that would look awful and send a deeply dispiriting message to the cards in that deck. Poor form. If the commander you picked has the partner keyword, you will be given the opportunity to pick a second commander. Note that any cards you search for in the software will be filtered by the color identity of your commanders; you will never be presented with cards that are color-incompatible with the deck.

Once you're looking at a deck, you'll want to add cards to it. From the Deck View, switch to the Database View by pressing Tab (they look very similar, but the titles in the omnibar and card list will change to reflect which you are looking at), then type in card names to filter the database. You can navigate to a card with the arrow keys and press Enter to add that card to your deck. Pressing spacebar when highlighting a card with a related card (such as an Adventure, a transformed or modal face, or a meld relationship) will show that other related card(s).

The left and right arrow keys will cycle through the tag list (arranged alphabetically). The current active tag is displayed in the top right of the window. You can press Enter to toggle the current active tag on the current active card, if that card is in your deck. Obviously, multiple tags can be added to any card, and I recommend added all relevant tags to a card as soon as it's added to the deck to make it easier to find and filter with later.

Speaking of filtering, that's a little too in-depth for this short summary. You can find more details about how to do it in the video below or in the project's [wiki](https://github.com/Endominus/Lieutenant/wiki/Card-Filtering-and-the-Omnibar).

https://www.youtube.com/watch?v=5LmR-bxYLo

## Contributing

Please don't read the source code. It's very ugly, and half-hungarian because I only partially finished that refactor, and I'm still not sure I exorcised all the demons from my first attempt at a PEG. If that doesn't scare you off, and there are features you think should be added to the software, that's great. I fully support your freedom to add them to your own copy. Have fun.
