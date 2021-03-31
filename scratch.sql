-- -- SQLite
-- SELECT 
--     name, 
--     -- card_text, 
--     -- mana_cost,
--     -- layout, 
--     types, 
--     -- supertypes, 
--     -- subtypes, 
--     color_identity, 
--     -- related_cards, 
--     -- power, 
--     -- toughness, 
--     cmc
-- FROM `cards`
-- INNER JOIN deck_contents
-- ON cards.name = deck_contents.card_name
-- WHERE deck_contents.deck = 1
-- -- AND power LIKE '%*%'
-- -- AND (cards.name LIKE '%ana%' OR cards.name LIKE '%wis%')
-- -- AND types LIKE '%artifact%' 
-- -- OR types LIKE '%land%'
-- -- AND cmc > 4
-- -- AND card_text LIKE '%you control%'
-- -- AND instr(mana_cost, 'R') = 0
-- -- AND instr(mana_cost, 'G') = 0 
-- -- AND instr(mana_cost, 'W') = 0 
-- ORDER BY name;

-- SELECT 
    -- name,
    -- legalities,
    -- card_text, 
    -- mana_cost,
    -- layout, 
    -- types, 
    -- color_identity, 
    -- related_cards, 
    -- power, 
    -- toughness, 
    -- cmc
    -- *
-- FROM cards
-- WHERE 1=1
-- AND legalities = "";
-- AND instr(color_identity, 'U') = 0 
-- AND instr(color_identity, 'B') = 0 
-- AND (types LIKE '%hydra%')
-- AND cmc < 4;
-- AND (cards.name LIKE '%ozi%')
-- AND (
--     (instr(mana_cost, 'B') > 0 AND instr(mana_cost, 'R') > 0 AND instr(mana_cost, 'U') > 0)
--     OR
--     (instr(mana_cost, 'W') > 0 AND instr(mana_cost, 'G') > 0)
-- )
-- AND ((instr(mana_cost, 'U') > 0 AND instr(mana_cost, 'R') > 0 AND instr(mana_cost, 'B') > 0) OR (instr(mana_cost, 'W') > 0 AND instr(mana_cost, 'G') > 0))
-- ORDER BY name;

-- SELECT DISTINCT layout from cards;

-- SELECT 
--         cmc, color_identity, legalities, loyalty, mana_cost, name, power, card_text, toughness, types, layout, related_cards, side
--         FROM cards WHERE card_text LIKE "%can't be blocked%";

-- SELECT * FROM cards WHERE name LIKE "%Isperia%";

-- DELETE from cards;
-- DELETE FROM decks;
-- DELETE FROM deck_contents;
-- DROP TABLE cards;
-- DROP TABLE deck_contents;

-- create table if not exists deck_contents (
--             id integer primary key,
--             card_name text not null,
--             deck integer not null,
--             tags text,
--             foreign key (deck) references decks(id),
--             unique (deck, card_name) on conflict ignore);

-- UPDATE cards 
-- SET related_cards = "Bruna, the Fading Light|Gisela, the Broken Blade" 
-- WHERE name = "Brisela, Voice of Nightmares";
-- UPDATE cards 
-- SET related_cards = "Bruna, the Fading Light|Brisela, Voice of Nightmares" 
-- WHERE name = "Gisela, the Broken Blade";
-- UPDATE cards 
-- SET related_cards = "Gisela, the Broken Blade|Brisela, Voice of Nightmares" 
-- WHERE name = "Bruna, the Fading Light";
-- UPDATE cards SET related_cards = "Graf Rats|Midnight Scavengers" WHERE name = "Chittering Host";
-- UPDATE cards SET related_cards = "Midnight Scavengers|Chittering Host" WHERE name = "Graf Rats";
-- UPDATE cards SET related_cards = "Hanweir Garrison|Hanweir, the Writhing Township" WHERE name = "Hanweir Battlements";
-- UPDATE cards SET related_cards = "Hanweir Battlements|Hanweir, the Writhing Township" WHERE name = "Hanweir Garrison";
-- UPDATE cards SET related_cards = "Hanweir Battlements|Hanweir Garrison" WHERE name = "Hanweir, the Writhing Township";
-- UPDATE cards SET related_cards = "Graf Rats|Chittering Host" WHERE name = "Midnight Scavengers";

-- UPDATE deck_contents SET tags = NULL WHERE tags = "";

-- SELECT * FROM deck_contents WHERE tags IS NOT NULL;

-- SELECT name,card_text,side,layout,related_cards,types 
-- FROM cards 
-- WHERE layout = "meld"
-- -- AND types LIKE '%Enchantment%'
-- -- AND name LIKE '%Rune%' 
-- ORDER BY name;

-- ALTER TABLE cards ADD COLUMN price REAL;
-- ALTER TABLE cards ADD COLUMN date_price_retrieved TEXT;