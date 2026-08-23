DELETE FROM rules
WHERE doc_id IN (
    SELECT id FROM documents
    WHERE id NOT IN (SELECT MAX(id) FROM documents GROUP BY doc_type)
);

DELETE FROM glossary
WHERE doc_id IN (
    SELECT id FROM documents
    WHERE id NOT IN (SELECT MAX(id) FROM documents GROUP BY doc_type)
);

DELETE FROM documents
WHERE id NOT IN (SELECT MAX(id) FROM documents GROUP BY doc_type);

CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_doc_type
ON documents(doc_type);
