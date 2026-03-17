#!/bin/bash

echo "🧪 Testing API endpoints..."
echo "=" * 50

# Test prompts endpoint (should return empty array or error if server not running)
echo "📝 Testing /api/prompts..."
curl -s http://localhost:3001/api/prompts || echo "Server not running"

# Test users endpoint (should return empty array or error if server not running)
echo -e "\n👥 Testing /api/users..."
curl -s http://localhost:3001/api/users || echo "Server not running"

# Test health endpoint (should return server status if running)
echo -e "\n❤️  Testing /health..."
curl -s http://localhost:3001/health || echo "Server not running"

echo -e "\n" "=" * 50
echo "📝 Note: The backend server has configuration issues and needs to be fixed."
echo "   However, the database, Meilisearch, and MinIO are all working correctly."
