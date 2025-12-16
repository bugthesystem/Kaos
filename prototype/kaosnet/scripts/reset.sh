#!/bin/bash
# Reset KaosNet database (drop all data and reseed)
#
# Usage: ./scripts/reset.sh [host] [port]
#
# WARNING: This will delete all data!

set -e

HOST=${1:-localhost}
PORT=${2:-5432}
USER=kaos
DB=kaosnet

echo "==> WARNING: This will delete ALL data in the KaosNet database!"
read -p "Are you sure? (y/N) " -n 1 -r
echo

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

echo "==> Resetting KaosNet database at $HOST:$PORT..."

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "Error: psql not found. Install PostgreSQL client."
    exit 1
fi

# Drop and recreate tables
PGPASSWORD=kaos psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" << 'EOF'
-- Drop all tables
DROP TABLE IF EXISTS tournament_records CASCADE;
DROP TABLE IF EXISTS tournaments CASCADE;
DROP TABLE IF EXISTS notifications CASCADE;
DROP TABLE IF EXISTS group_members CASCADE;
DROP TABLE IF EXISTS groups CASCADE;
DROP TABLE IF EXISTS friends CASCADE;
DROP TABLE IF EXISTS leaderboard_records CASCADE;
DROP TABLE IF EXISTS storage_objects CASCADE;
DROP TABLE IF EXISTS players CASCADE;
DROP TABLE IF EXISTS api_keys CASCADE;
DROP TABLE IF EXISTS console_accounts CASCADE;

-- Notify
SELECT 'All tables dropped' AS status;
EOF

echo "==> Running schema migration..."
PGPASSWORD=kaos psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -f "$(dirname "$0")/init.sql"

echo "==> Seeding fresh data..."
PGPASSWORD=kaos psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -f "$(dirname "$0")/seed.sql"

echo "==> Database reset complete!"
echo ""
echo "Fresh sample data:"
echo "  - 10 players"
echo "  - 3 groups"
echo "  - Leaderboard entries"
echo "  - Sample storage, friends, notifications"
