#!/bin/bash

# SSL Certificate Setup Script for BitQuan Monitoring
# This script generates self-signed certificates for development
# For production, use certificates from a proper CA

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="${SCRIPT_DIR}/certs"

echo "🔐 Setting up SSL certificates for BitQuan monitoring..."

# Create certificates directory
mkdir -p "${CERTS_DIR}"

# Generate private key
echo "📋 Generating private key..."
openssl genrsa -out "${CERTS_DIR}/bitquan.key" 4096

# Generate certificate signing request
echo "📝 Generating certificate signing request..."
openssl req -new -key "${CERTS_DIR}/bitquan.key" -out "${CERTS_DIR}/bitquan.csr" -subj "/C=US/ST=CA/L=San Francisco/O=BitQuan/CN=bitquan.local"

# Generate self-signed certificate
echo "📜 Generating self-signed certificate..."
openssl x509 -req -days 365 -in "${CERTS_DIR}/bitquan.csr" -signkey "${CERTS_DIR}/bitquan.key" -out "${CERTS_DIR}/bitquan.crt"

# Generate certificate for localhost
echo "📝 Generating localhost certificate..."
openssl req -new -x509 -days 365 -nodes -out "${CERTS_DIR}/localhost.crt" -keyout "${CERTS_DIR}/localhost.key" -subj "/C=US/ST=CA/L=San Francisco/O=BitQuan/CN=localhost"

# Create certificate bundle for Traefik
echo "🔗 Creating certificate bundle..."
cat "${CERTS_DIR}/bitquan.crt" "${CERTS_DIR}/localhost.crt" > "${CERTS_DIR}/bundle.crt"

# Copy private key to match bundle name
cp "${CERTS_DIR}/bitquan.key" "${CERTS_DIR}/bundle.key"

# Set proper permissions
echo "🔒 Setting secure permissions..."
chmod 600 "${CERTS_DIR}"/*.key
chmod 644 "${CERTS_DIR}"/*.crt
chmod 644 "${CERTS_DIR}"/*.csr

# Create ACME JSON file for Let's Encrypt (empty for development)
echo "📄 Creating ACME storage file..."
echo '{}' > "${CERTS_DIR}/acme.json"
chmod 600 "${CERTS_DIR}/acme.json"

# Clean up CSR
rm -f "${CERTS_DIR}/bitquan.csr"

echo "✅ SSL certificates generated successfully!"
echo ""
echo "📁 Certificate files created in: ${CERTS_DIR}"
echo "   - bitquan.key (private key)"
echo "   - bitquan.crt (certificate)"
echo "   - localhost.key (localhost private key)"
echo "   - localhost.crt (localhost certificate)"
echo "   - bundle.crt (certificate bundle)"
echo "   - bundle.key (bundle private key)"
echo "   - acme.json (Let's Encrypt storage)"
echo ""
echo "⚠️  WARNING: These are self-signed certificates for development only!"
echo "   For production, use certificates from a trusted Certificate Authority."
echo ""
echo "🚀 You can now start the monitoring stack with:"
echo "   cd ${SCRIPT_DIR}"
echo "   docker-compose up -d"
