#!/bin/bash

# JWT Secret Setup Script for BitQuan
# This script generates secure JWT secrets and password hashes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="${SCRIPT_DIR}/config"
JWT_CONFIG="${SCRIPT_DIR}/jwt.toml"
JWT_ENV="${SCRIPT_DIR}/jwt.env"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Generate secure random secret
generate_secret() {
    openssl rand -hex 32
}

# Generate Argon2id password hash
generate_password_hash() {
    local password="$1"

    # Check if argon2 command is available
    if command -v argon2 &> /dev/null; then
        echo -n "$password" | argon2 -id -t 2 -m 19456 -p 1 -l 32 | sed 's/^$argon2id\$v=19\$m=19456,t=2,p=1\$\([a-zA-Z0-9+/]\+\)\$\([a-zA-Z0-9+/]\+\)\$$/\$\1\$\2/'
    else
        log_warning "argon2 not found, using default hash. Please install argon2 for better security."
        # Fallback to a predefined hash (not recommended for production)
        echo "\$argon2id\$v=19\$m=19456,t=2,p=1\$Q1jyewUdmakjPNt4n2LZFg\$vxXidOOsjayYC+M4R0XyB8FZZ9+tLfGNZtla0nGkh4A"
    fi
}

# Create configuration directory
create_config_directory() {
    log_info "Creating configuration directory..."
    mkdir -p "$CONFIG_DIR"
}

# Generate JWT environment file
generate_jwt_env() {
    log_info "Generating JWT environment variables..."

    local jwt_secret
    local admin_password
    local admin_password_hash

    # Generate secure JWT secret
    jwt_secret=$(generate_secret)
    log_info "Generated JWT secret: ${jwt_secret:0:8}..."

    # Prompt for admin password
    echo ""
    log_info "Enter admin password for JWT authentication:"
    echo -n "Password (will be hidden): "
    read -r -s admin_password
    echo ""

    if [[ -z "$admin_password" ]]; then
        log_error "Password cannot be empty"
        exit 1
    fi

    if [[ ${#admin_password} -lt 8 ]]; then
        log_error "Password must be at least 8 characters long"
        exit 1
    fi

    # Generate password hash
    admin_password_hash=$(generate_password_hash "$admin_password")
    log_success "Generated password hash"

    # Create .env file
    cat > "$JWT_ENV" << EOF
# JWT Authentication Configuration
# Generated on $(date)
# DO NOT commit this file to version control!

# JWT signing secret (32 bytes, hex encoded)
JWT_SECRET=${jwt_secret}

# Admin user credentials
JWT_ADMIN_USERNAME=admin
JWT_ADMIN_PASSWORD_HASH=${admin_password_hash}

# Security settings
JWT_EXPIRATION_HOURS=24
JWT_REFRESH_TOKEN_EXPIRY=7d
EOF

    chmod 600 "$JWT_ENV"
    log_success "Created $JWT_ENV with secure permissions"
}

# Generate JWT configuration template
generate_jwt_config_template() {
    log_info "Creating JWT configuration template..."

    cat > "${JWT_CONFIG}.template" << 'EOF'
# JWT Authentication Configuration Template
# Copy this file to jwt.toml and set the values from jwt.env

# IMPORTANT: Replace the placeholders with actual values from jwt.env
secret = "${JWT_SECRET}"

[[users]]
username = "${JWT_ADMIN_USERNAME}"
password_hash = "${JWT_ADMIN_PASSWORD_HASH}"
role = "admin"

# You can add more users here:
# [[users]]
# username = "viewer"
# password_hash = "$argon2id$v=19$m=19456,t=2,p=1$Q1jyewUdmakjPNt4n2LZFg$vxXidOOsjayYC+M4R0XyB8FZZ9+tLfGNZtla0nGkh4A"
# role = "viewer"
EOF

    log_success "Created ${JWT_CONFIG}.template"
}

# Validate generated configuration
validate_configuration() {
    log_info "Validating generated configuration..."

    if [[ ! -f "$JWT_ENV" ]]; then
        log_error "JWT environment file not created"
        return 1
    fi

    # Source the environment file
    source "$JWT_ENV"

    if [[ -z "${JWT_SECRET:-}" ]]; then
        log_error "JWT_SECRET not set"
        return 1
    fi

    if [[ ${#JWT_SECRET} -ne 64 ]]; then
        log_error "JWT_SECRET must be 64 characters (32 bytes hex)"
        return 1
    fi

    if [[ -z "${JWT_ADMIN_PASSWORD_HASH:-}" ]]; then
        log_error "JWT_ADMIN_PASSWORD_HASH not set"
        return 1
    fi

    log_success "Configuration validation passed"
}

# Show usage instructions
show_usage_instructions() {
    log_success "JWT secrets generated successfully!"
    echo ""
    echo "📁 Generated files:"
    echo "   - $JWT_ENV (environment variables - keep secret!)"
    echo "   - ${JWT_CONFIG}.template (configuration template)"
    echo ""
    echo "🔐 Next steps:"
    echo "   1. Deploy with environment variables:"
    echo "      docker run --env-file $JWT_ENV your-image"
    echo ""
    echo "   2. Or create jwt.toml from template:"
    echo "      envsubst < ${JWT_CONFIG}.template > jwt.toml"
    echo ""
    echo "   3. Or use in docker-compose:"
    echo "      env_file: $JWT_ENV"
    echo ""
    echo "   4. NEVER commit $JWT_ENV to version control!"
    echo ""
    log_warning "Security reminders:"
    echo "   - Keep the JWT_ENV file secure and backed up"
    echo "   - Rotate JWT secrets periodically"
    echo "   - Use strong passwords (at least 8 characters)"
    echo "   - Monitor for unauthorized access attempts"
}

# Main setup function
main() {
    log_info "Setting up JWT authentication secrets..."

    create_config_directory
    generate_jwt_env
    generate_jwt_config_template
    validate_configuration
    show_usage_instructions
}

# Handle script interruption
trap 'log_error "Setup interrupted"; exit 1' INT TERM

# Run main function
main "$@"
