DELETE FROM cards
WHERE printings IS NULL
  AND name IN (SELECT name FROM cards WHERE printings IS NOT NULL);
