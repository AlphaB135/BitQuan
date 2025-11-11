#!/bin/bash
# BitQuan Dashboard - Quick Start Script

cd "$(dirname "$0")"

echo "🚀 Starting BitQuan Testnet Dashboard..."
echo ""

# Check if port 8080 is available
if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null ; then
    echo "⚠️  Port 8080 is already in use!"
    echo "   Killing existing process..."
    lsof -ti:8080 | xargs kill -9 2>/dev/null
    sleep 1
fi

# Start server
python3 server.py &
PID=$!

echo ""
echo "✅ Dashboard started!"
echo "📍 PID: $PID"
echo "🌐 URL: http://localhost:8080"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Wait for Ctrl+C
trap "kill $PID 2>/dev/null; echo ''; echo '👋 Dashboard stopped!'; exit 0" INT
wait $PID
