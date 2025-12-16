#!/bin/bash
# Seed KaosNet database with sample data
#
# Usage: ./scripts/seed.sh [host] [port]
#
# Defaults to localhost:5432

set -e

HOST=${1:-localhost}
PORT=${2:-5432}
USER=kaos
DB=kaosnet

echo "==> Seeding KaosNet database at $HOST:$PORT..."

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "Error: psql not found. Install PostgreSQL client."
    exit 1
fi

# Run seed script
PGPASSWORD=kaos psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -f "$(dirname "$0")/seed.sql"

echo "==> Sample data seeded successfully!"
echo ""
echo "Sample accounts created:"
echo "  - 10 players (user_001 to user_010)"
echo "  - 3 groups (Elite Gamers, Casual Crew, Speed Run Masters)"
echo "  - Leaderboard entries for kaos_io_highscores, weekly_scores, asteroids_highscores"
echo "  - Sample storage objects, friends, notifications"
echo ""
echo "Login to console: admin / admin"
