CREATE TABLE journals (
    id BIGINT PRIMARY KEY AUTO_INCREMENT
);

INSERT INTO journals (id) VALUES (1);

ALTER TABLE journal_entries
    MODIFY COLUMN id BIGINT NOT NULL AUTO_INCREMENT,
    ADD COLUMN journal_id BIGINT NULL;

UPDATE journal_entries SET journal_id = 1;

ALTER TABLE journal_entries
    MODIFY COLUMN journal_id BIGINT NOT NULL,
    ADD CONSTRAINT fk_journal_entries_journal
        FOREIGN KEY (journal_id) REFERENCES journals(id);
