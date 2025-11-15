#!/bin/bash

# BitQuan GUI Development Helper Script

echo "🚀 BitQuan GUI Development Helper"
echo "================================"

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo "❌ Error: package.json not found. Please run this script from the bitquan-gui directory."
    exit 1
fi

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    npm install
fi

# Check if TypeScript is available
if ! command -v npx &> /dev/null; then
    echo "❌ Error: npx not found. Please install Node.js and npm."
    exit 1
fi

echo "🔍 Running TypeScript type check..."
npx tsc --noEmit

if [ $? -eq 0 ]; then
    echo "✅ TypeScript check passed!"
else
    echo "❌ TypeScript errors found. Please fix them before continuing."
    exit 1
fi

echo "🎨 Building CSS with Tailwind..."
npx tailwindcss -i ./index.css -o ./dist.css --watch &

TAILWIND_PID=$!

echo "🌐 Starting development server..."
echo "📱 The app will be available at http://localhost:8080"
echo "🔧 Tauri dev mode will also start automatically"
echo ""
echo "Press Ctrl+C to stop all services"

# Start the development server
npm run dev

# Clean up Tailwind process on exit
kill $TAILWIND_PID 2>/dev/null