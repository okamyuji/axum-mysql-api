CREATE TABLE journals (
    id BIGINT PRIMARY KEY AUTO_INCREMENT
);

CREATE TABLE journal_entries (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    journal_id BIGINT NOT NULL,
    account_code VARCHAR(20) NOT NULL,
    amount_cents BIGINT NOT NULL,
    description VARCHAR(255) NOT NULL,
    FOREIGN KEY (journal_id) REFERENCES journals(id)
);

INSERT INTO journals (id) VALUES (1);
INSERT INTO journal_entries (id, journal_id, account_code, amount_cents, description) VALUES
    (1, 1, '1000', 12500, 'Cash receipt'),
    (2, 1, '4000', -12500, 'Sales revenue');
