#!/bin/bash

# Secure Monitoring Deployment Script for BitQuan
# This script deploys the monitoring stack with security configurations

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

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

# Security check functions
check_dependencies() {
    log_info "Checking dependencies..."

    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed or not in PATH"
        exit 1
    fi

    if ! command -v openssl &> /dev/null; then
        log_error "OpenSSL is not installed or not in PATH"
        exit 1
    fi

    log_success "All dependencies are available"
}

check_environment() {
    log_info "Checking environment configuration..."

    if [[ ! -f ".env" ]]; then
        log_warning ".env file not found. Creating from template..."
        if [[ -f ".env.example" ]]; then
            cp .env.example .env
            log_warning "Please edit .env file with secure passwords before continuing"
            log_warning "Current passwords are DEFAULT and INSECURE"
            echo ""
            echo "Edit the .env file and press Enter to continue..."
            read -r
        else
            log_error ".env.example file not found"
            exit 1
        fi
    fi

    # Load environment variables
    source .env

    # Check if default passwords are still being used
    if [[ "${GRAFANA_ADMIN_PASSWORD:-}" == "admin123" ]]; then
        log_error "Default Grafana password detected! Please change GRAFANA_ADMIN_PASSWORD in .env"
        exit 1
    fi

    if [[ "${TRAEFIK_PASSWORD:-}" == "securepassword123" ]]; then
        log_error "Default Traefik password detected! Please change TRAEFIK_PASSWORD in .env"
        exit 1
    fi

    log_success "Environment configuration is valid"
}

setup_ssl_certificates() {
    log_info "Setting up SSL certificates..."

    if [[ ! -d "certs" ]] || [[ ! -f "certs/bundle.crt" ]]; then
        log_info "SSL certificates not found. Generating new certificates..."
        ./setup-ssl.sh
    else
        log_info "SSL certificates already exist"
    fi
}

validate_ssl_certificates() {
    log_info "Validating SSL certificates..."

    if [[ ! -f "certs/bundle.crt" ]] || [[ ! -f "certs/bundle.key" ]]; then
        log_error "SSL certificate files not found"
        exit 1
    fi

    # Check certificate validity
    if openssl x509 -checkend 0 -noout -in "certs/bundle.crt" > /dev/null 2>&1; then
        log_success "SSL certificates are valid"
    else
        log_error "SSL certificates are expired or invalid"
        exit 1
    fi
}

check_network_security() {
    log_info "Checking network security configuration..."

    # Verify monitoring network is internal
    if docker network ls | grep -q "monitoring.*internal.*true"; then
        log_success "Monitoring network is properly isolated"
    else
        log_warning "Monitoring network may not be properly isolated"
    fi

    # Check for exposed ports (should be minimal)
    local exposed_ports
    exposed_ports=$(grep -E "^\s*-\s*\"\d+:" docker-compose.yml | wc -l)

    if [[ $exposed_ports -le 3 ]]; then
        log_success "Minimal port exposure detected ($exposed_ports ports)"
    else
        log_warning "Many ports exposed ($exposed_ports ports). Consider reducing exposure."
    fi
}

deploy_monitoring() {
    log_info "Deploying secure monitoring stack..."

    # Pull latest images
    log_info "Pulling Docker images..."
    docker-compose pull

    # Start services
    log_info "Starting monitoring services..."
    docker-compose up -d

    # Wait for services to be ready
    log_info "Waiting for services to be ready..."
    sleep 30

    # Check service health
    check_service_health
}

check_service_health() {
    log_info "Checking service health..."

    local services=("traefik" "grafana" "prometheus" "alertmanager")
    local unhealthy_services=()

    for service in "${services[@]}"; do
        if docker-compose ps "$service" | grep -q "Up (healthy)\|Up"; then
            log_success "$service is running"
        else
            log_error "$service is not healthy or not running"
            unhealthy_services+=("$service")
        fi
    done

    if [[ ${#unhealthy_services[@]} -gt 0 ]]; then
        log_error "Some services are unhealthy: ${unhealthy_services[*]}"
        log_info "Check logs with: docker-compose logs [service-name]"
        exit 1
    fi
}

show_access_info() {
    log_success "Secure monitoring stack deployed successfully!"
    echo ""
    echo "🔐 Access URLs (with HTTPS and authentication):"
    echo "   Grafana:        https://grafana.localhost"
    echo "   Prometheus:     https://prometheus.localhost"
    echo "   AlertManager:   https://alerts.localhost"
    echo "   Traefik:        https://traefik.localhost"
    echo ""
    echo "👤 Default Authentication:"
    echo "   Username:       admin"
    echo "   Grafana Pass:    Check .env file (GRAFANA_ADMIN_PASSWORD)"
    echo "   Traefik Pass:    Check .env file (TRAEFIK_PASSWORD)"
    echo ""
    echo "🔍 Health Check:"
    echo "   https://grafana.localhost/health"
    echo ""
    echo "📋 Management Commands:"
    echo "   View logs:       docker-compose logs -f [service-name]"
    echo "   Stop stack:      docker-compose down"
    echo "   Update stack:    docker-compose pull && docker-compose up -d"
    echo ""
    log_warning "Remember to:"
    echo "   1. Change default passwords in .env file"
    echo "   2. Use proper SSL certificates in production"
    echo "   3. Configure firewalls to restrict access"
    echo "   4. Set up monitoring and alerting for the monitoring stack itself"
}

# Main deployment function
main() {
    log_info "Starting secure BitQuan monitoring deployment..."

    check_dependencies
    check_environment
    setup_ssl_certificates
    validate_ssl_certificates
    check_network_security
    deploy_monitoring
    show_access_info

    log_success "Secure monitoring deployment completed!"
}

# Handle script interruption
trap 'log_error "Deployment interrupted"; exit 1' INT TERM

# Run main function
main "$@"
