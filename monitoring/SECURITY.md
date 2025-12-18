# BitQuan Monitoring Security Guide

This document outlines the security measures implemented for the BitQuan monitoring infrastructure.

## 🛡️ Security Features

### 1. Network Isolation
- **Internal Network**: All monitoring services run in an isolated Docker network (`monitoring`) with `internal: true`
- **Reverse Proxy Only**: Only Traefik reverse proxy is exposed to external network
- **No Direct Port Exposure**: Services like Prometheus, AlertManager, and Node Exporter are not directly accessible from host

### 2. Authentication & Authorization
- **HTTP Basic Auth**: All services require authentication via Traefik middlewares
- **Separate Credentials**: Each service can have different authentication requirements
- **Role-Based Access**: Grafana supports anonymous view access with viewer role

### 3. TLS/SSL Encryption
- **HTTPS Only**: All external traffic is forced to use HTTPS
- **Modern TLS**: TLS 1.2+ with strong cipher suites
- **Certificate Management**: Support for Let's Encrypt and self-signed certificates

### 4. Security Headers
- **CSP**: Content Security Policy to prevent XSS attacks
- **HSTS**: HTTP Strict Transport Security
- **X-Frame-Options**: Prevent clickjacking
- **X-Content-Type-Options**: Prevent MIME-type sniffing
- **Referrer Policy**: Control referrer information

### 5. Rate Limiting & IP Filtering
- **Rate Limiting**: 100 requests/minute average, 200 burst
- **IP Whitelisting**: Restrict access to private networks and localhost
- **Brute Force Protection**: Via authentication and rate limiting

## 🔐 Authentication Setup

### Default Users
- **Admin**: `admin` (change password in .env)
- **Viewer**: `viewer` (view-only access to Grafana)

### Password Hashes
Passwords are stored as BCrypt hashes. Default hashes:
- Admin: `$2a$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBdXwt.Gtu5xg6` (password: `securepassword123`)
- Viewer: `$2a$12$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy` (password: `viewonly123`)

### Generating New Passwords
```bash
# Using htpasswd
htpasswd -nbB username password

# Using Docker
docker run --rm -it httpd:2.4-alpine htpasswd -nbB username password
```

## 🚀 Deployment Security

### Environment Variables
All sensitive data is stored in environment variables (`.env` file):
- `GRAFANA_ADMIN_PASSWORD`: Grafana admin password
- `TRAEFIK_PASSWORD`: Traefik dashboard password
- Various SSL and network configuration options

### SSL Certificates
- **Development**: Self-signed certificates via `setup-ssl.sh`
- **Production**: Use certificates from trusted CA or Let's Encrypt

### Service Exposure
Only these ports are exposed:
- **80**: HTTP (redirects to HTTPS)
- **443**: HTTPS with authentication
- **8080**: Traefik dashboard (internal, with auth)

## 📋 Security Checklist

### Before Deployment
- [ ] Change default passwords in `.env` file
- [ ] Generate new SSL certificates for production
- [ ] Configure firewall rules to restrict access
- [ ] Set up proper DNS for SSL certificates
- [ ] Review and adjust IP whitelist ranges

### After Deployment
- [ ] Verify all services require authentication
- [ ] Check that only HTTPS is accessible
- [ ] Test rate limiting functionality
- [ ] Verify security headers are present
- [ ] Monitor for unauthorized access attempts

### Ongoing Maintenance
- [ ] Regularly rotate passwords
- [ ] Update SSL certificates before expiration
- [ ] Monitor Docker image updates
- [ ] Review access logs regularly
- [ ] Backup SSL certificates and configuration

## 🔍 Access URLs

All services are accessible via HTTPS with authentication:

- **Grafana**: `https://grafana.localhost` (admin/ + password from .env)
- **Prometheus**: `https://prometheus.localhost` (basic auth)
- **AlertManager**: `https://alerts.localhost` (basic auth)
- **Traefik Dashboard**: `https://traefik.localhost` (basic auth)

## 🛠️ Management Commands

```bash
# Deploy with security
./deploy-secure.sh

# Check service status
docker-compose ps

# View logs
docker-compose logs -f [service-name]

# Update services
docker-compose pull && docker-compose up -d

# Stop all services
docker-compose down

# Rotate passwords
# 1. Update .env file
# 2. Update traefik/dynamic/middlewares.yml
# 3. docker-compose restart traefik
```

## 🚨 Security Alerts

Monitor for these security events:
- Multiple failed authentication attempts
- Access from unauthorized IP ranges
- SSL certificate expiration
- Unusual traffic patterns
- Service downtime or anomalies

## 📚 Additional Resources

- [Traefik Security Documentation](https://doc.traefik.io/traefik/security/)
- [Grafana Security Best Practices](https://grafana.com/docs/grafana/latest/administration/security/)
- [Prometheus Security Guidelines](https://prometheus.io/docs/guides/security/)
- [Docker Security Best Practices](https://docs.docker.com/engine/security/)
