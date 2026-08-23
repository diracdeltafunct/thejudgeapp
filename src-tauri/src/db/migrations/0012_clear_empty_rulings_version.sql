DELETE FROM documents
WHERE doc_type = 'rulings'
  AND NOT EXISTS (SELECT 1 FROM card_rulings LIMIT 1);
