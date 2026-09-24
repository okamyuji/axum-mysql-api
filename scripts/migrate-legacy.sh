#!/bin/sh
set -eu

: "${MYSQL_DATABASE:?Set MYSQL_DATABASE}"
: "${MYSQL_USER:?Set MYSQL_USER}"
: "${MYSQL_PWD:?Set MYSQL_PWD}"

script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
mysql_query() {
    mysql --host="${MYSQL_HOST:-127.0.0.1}" --port="${MYSQL_PORT:-3306}" \
        --user="$MYSQL_USER" --database="$MYSQL_DATABASE" --batch --skip-column-names \
        --execute="$1"
}

state=$(mysql_query "SELECT CONCAT(
    (SELECT COUNT(*) FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'journals'), ':',
    (SELECT COUNT(*) FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'journal_entries'), ':',
    (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'journal_entries' AND COLUMN_NAME = 'journal_id'))")

case "$state" in
    1:1:1)
        complete=$(mysql_query "SELECT
            (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'journal_entries' AND COLUMN_NAME = 'journal_id' AND IS_NULLABLE = 'NO') = 1
            AND (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'journal_entries' AND COLUMN_NAME = 'id' AND EXTRA LIKE '%auto_increment%') = 1
            AND (SELECT COUNT(*) FROM information_schema.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'journal_entries' AND COLUMN_NAME = 'journal_id' AND REFERENCED_TABLE_NAME = 'journals') = 1")
        if [ "$complete" = 1 ]; then echo 'Already migrated'; exit 0; fi
        echo 'Partially migrated schema; no changes made' >&2
        exit 1
        ;;
    0:1:0) ;;
    *) echo "Unexpected or partially migrated schema ($state); no changes made" >&2; exit 1 ;;
esac

seed=$(mysql_query "SELECT CASE WHEN COUNT(*) = 2
    AND SUM(id = 1 AND account_code = '1000' AND amount_cents = 12500) = 1
    AND SUM(id = 2 AND account_code = '4000' AND amount_cents = -12500) = 1
    THEN 1 ELSE 0 END FROM journal_entries")
if [ "$seed" != 1 ]; then
    echo 'Legacy data differs from the two seeded entries; no changes made' >&2
    exit 1
fi

mysql --host="${MYSQL_HOST:-127.0.0.1}" --port="${MYSQL_PORT:-3306}" \
    --user="$MYSQL_USER" --database="$MYSQL_DATABASE" < "$script_dir/upgrade-legacy.sql"
echo 'Legacy schema migrated'
