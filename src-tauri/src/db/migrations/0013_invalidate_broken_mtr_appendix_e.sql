DELETE FROM rules
WHERE doc_id IN (
    SELECT id FROM documents
    WHERE doc_type = 'mtr' AND version = '20260228'
);

DELETE FROM glossary
WHERE doc_id IN (
    SELECT id FROM documents
    WHERE doc_type = 'mtr' AND version = '20260228'
);

DELETE FROM documents
WHERE doc_type = 'mtr' AND version = '20260228';
