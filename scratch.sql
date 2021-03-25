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

SELECT 
    name, 
    -- card_text, 
    mana_cost,
    layout, 
    types, 
    supertypes, 
    subtypes, 
    color_identity, 
    related_cards, 
    power, 
    toughness, 
    cmc
FROM `cards`
WHERE 1=1
AND instr(color_identity, 'U') = 0 
AND instr(color_identity, 'B') = 0 
AND ((subtypes LIKE '%hydra%'))
AND cmc < 4
-- AND (cards.name LIKE '%ozi%')
-- -- AND (
-- --     (instr(mana_cost, 'B') > 0 AND instr(mana_cost, 'R') > 0 AND instr(mana_cost, 'U') > 0)
-- --     OR
-- --     (instr(mana_cost, 'W') > 0 AND instr(mana_cost, 'G') > 0)
-- -- )
-- -- AND ((instr(mana_cost, 'U') > 0 AND instr(mana_cost, 'R') > 0 AND instr(mana_cost, 'B') > 0) OR (instr(mana_cost, 'W') > 0 AND instr(mana_cost, 'G') > 0))
ORDER BY name;

-- SELECT DISTINCT layout FROM cards;

-- SELECT * FROM cards WHERE name = "Karn Liberated";
