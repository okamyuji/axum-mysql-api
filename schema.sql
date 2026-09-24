CREATE TABLE journal_entries (
    id BIGINT PRIMARY KEY,
    account_code VARCHAR(20) NOT NULL,
    amount_cents BIGINT NOT NULL,
    description VARCHAR(255) NOT NULL
);

INSERT INTO journal_entries (id, account_code, amount_cents, description) VALUES
    (1, '1000', 12500, 'Cash receipt'),
    (2, '4000', -12500, 'Sales revenue');
