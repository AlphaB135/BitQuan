# BitQuan Deployment Automation

This directory contains comprehensive infrastructure-as-code and deployment automation for BitQuan blockchain nodes across multiple environments.

## 🏗️ Architecture Overview

### Environments
- **Staging**: Development and testing environment (2 replicas)
- **Testnet**: Public test network (3 replicas) 
- **Mainnet**: Production network (5+ replicas with auto-scaling)

### Technology Stack
- **Infrastructure**: Terraform + AWS EKS
- **Containerization**: Docker + Kubernetes
- **CI/CD**: GitHub Actions
- **Monitoring**: Prometheus + Grafana
- **Security**: IAM roles, network policies, secrets management

## 📁 Directory Structure

```
deploy/
├── terraform/           # Infrastructure as code
│   ├── main.tf         # Main Terraform configuration
│   ├── variables.tf    # Input variables
│   ├── outputs.tf      # Output values
│   └── modules/       # Reusable Terraform modules
├── kubernetes/         # Kubernetes manifests
│   ├── namespaces.yaml # Namespaces and RBAC
│   ├── configmaps.yaml # Application configuration
│   ├── deployments.yaml # Pod deployments
│   ├── services.yaml   # Services and networking
│   └── ingress.yaml    # Load balancer configuration
├── scripts/           # Deployment and utility scripts
│   ├── production-deploy.sh # Main deployment script
│   ├── build-release.sh     # Release build script
│   └── deploy-cluster.sh    # Cluster deployment script
└── configs/           # Environment-specific configurations
    ├── staging.toml
    ├── testnet.toml
    └── mainnet.toml
```

## 🚀 Quick Start

### Prerequisites

1. **Required Tools**:
   ```bash
   # Install CLI tools
   brew install terraform kubectl helm docker
   
   # AWS CLI and authentication
   aws configure
   ```

2. **Permissions**:
   - AWS account with EKS, EC2, S3, DynamoDB permissions
   - GitHub repository access
   - Container registry permissions

### Deploy to Staging

```bash
# Quick deployment to staging
./deploy/scripts/production-deploy.sh staging us-west-2 latest

# Deploy specific version
./deploy/scripts/production-deploy.sh staging us-west-2 v1.2.3
```

### Deploy to Testnet

```bash
# Deploy to testnet (requires manual approval in GitHub Actions)
./deploy/scripts/production-deploy.sh testnet us-west-2 v1.2.3
```

### Deploy to Mainnet

```bash
# Mainnet deployment (requires multiple approvals)
./deploy/scripts/production-deploy.sh mainnet us-west-2 v1.2.3

# Force deployment (skip health checks)
./deploy/scripts/production-deploy.sh mainnet us-west-2 v1.2.3 true
```

## 🔧 Infrastructure Management

### Terraform Operations

```bash
# Initialize Terraform
cd deploy/terraform
terraform init

# Plan infrastructure changes
terraform plan -var="environment=staging" -var="aws_region=us-west-2"

# Apply changes
terraform apply -var="environment=staging" -var="aws_region=us-west-2"

# Destroy infrastructure (DANGEROUS)
terraform destroy -var="environment=staging" -var="aws_region=us-west-2"
```

### Kubernetes Operations

```bash
# Get cluster credentials
aws eks update-kubeconfig --region us-west-2 --name bitquan-staging

# View deployment status
kubectl get deployments -n bitquan-staging

# Scale deployment
kubectl scale deployment bitquan-node --replicas=5 -n bitquan-staging

# View logs
kubectl logs -f deployment/bitquan-node -n bitquan-staging

# Port forward for local testing
kubectl port-forward service/bitquan-node 8080:80 -n bitquan-staging
```

## 🔄 CI/CD Pipeline

### GitHub Actions Workflow

The deployment pipeline includes these stages:

1. **Security Scan**
   - Cargo audit for vulnerabilities
   - Trivy filesystem scan
   - Dependency checks

2. **Build and Test**
   - Multi-platform compilation
   - Unit and integration tests
   - Docker image building

3. **Infrastructure Validation**
   - Terraform format and validate
   - Plan review
   - Security compliance

4. **Deployment**
   - Staging: Automatic
   - Testnet: Manual approval
   - Mainnet: Multiple approvals + verification

### Manual Deployment

```bash
# Trigger deployment via GitHub CLI
gh workflow run production-deploy.yml \
  --field environment=testnet \
  --field region=us-west-2 \
  --field force_deploy=false
```

## 📊 Monitoring and Observability

### Access Monitoring Stack

```bash
# Start monitoring services
cd monitoring
docker-compose up -d

# Access dashboards
# Grafana: http://localhost:3000 (admin/admin123)
# Prometheus: http://localhost:9090
# AlertManager: http://localhost:9093
```

### Key Metrics

- **Mining Metrics**: Hashrate, blocks mined, pool efficiency
- **Network Metrics**: Peer connections, sync status, block propagation
- **System Metrics**: CPU, memory, disk usage, error rates
- **Application Metrics**: HTTP requests, WebSocket connections, API latency

### Alerting

Critical alerts are configured for:
- Node downtime
- High error rates
- Low hashrate
- Storage issues
- Network connectivity problems

## 🔒 Security Configuration

### Network Security

- **VPC**: Private subnets with NAT gateways
- **Security Groups**: Restrictive inbound/outbound rules
- **Network Policies**: Kubernetes network segmentation
- **TLS**: End-to-end encryption for all communications

### Access Control

- **IAM Roles**: Least privilege access patterns
- **RBAC**: Kubernetes role-based access control
- **Secrets Management**: Encrypted secrets in Kubernetes
- **Audit Logging**: Comprehensive audit trails

### Compliance

- **SOC 2**: Security controls and monitoring
- **GDPR**: Data protection and privacy
- **PCI DSS**: Payment card industry compliance (if applicable)

## 🛠️ Troubleshooting

### Common Issues

1. **Deployment Fails**
   ```bash
   # Check deployment status
   kubectl get deployment bitquan-node -n bitquan-staging -o wide
   
   # Check pod logs
   kubectl logs -f deployment/bitquan-node -n bitquan-staging
   
   # Describe pod for errors
   kubectl describe pod -l app=bitquan-node -n bitquan-staging
   ```

2. **Health Checks Fail**
   ```bash
   # Test health endpoint directly
   kubectl port-forward service/bitquan-node 8080:80 -n bitquan-staging
   curl http://localhost:8080/health
   
   # Check resource limits
   kubectl top pods -n bitquan-staging
   ```

3. **Infrastructure Issues**
   ```bash
   # Check Terraform state
   terraform show
   
   # Validate AWS resources
   aws eks describe-cluster --name bitquan-staging
   ```

### Rollback Procedures

```bash
# Quick rollback via script
./deploy/scripts/production-deploy.sh rollback

# Manual Kubernetes rollback
kubectl rollout undo deployment/bitquan-node -n bitquan-staging

# Terraform rollback
terraform apply -var="environment=staging" -target=module.eks
```

## 📈 Performance Optimization

### Scaling Configuration

- **Horizontal Pod Autoscaler**: CPU/Memory-based scaling
- **Cluster Autoscaler**: Automatic node scaling
- **Load Balancing**: Intelligent traffic distribution
- **Caching**: Redis for frequently accessed data

### Resource Allocation

| Environment | Replicas | CPU per Pod | Memory per Pod | Storage |
|-------------|-----------|-------------|---------------|---------|
| Staging     | 2         | 1 core      | 2 Gi          | 50 Gi   |
| Testnet     | 3         | 2 cores     | 4 Gi          | 100 Gi  |
| Mainnet     | 5+        | 4 cores     | 8 Gi          | 500 Gi  |

## 🔄 Maintenance Procedures

### Regular Maintenance

1. **Daily**: Monitor alerts and performance metrics
2. **Weekly**: Review logs and update configurations
3. **Monthly**: Apply security patches and updates
4. **Quarterly**: Disaster recovery testing and audits

### Backup Strategy

- **Data Backups**: Automated daily snapshots to S3
- **Configuration Backups**: Git version control
- **Infrastructure Backups**: Terraform state management
- **Disaster Recovery**: Multi-region replication

## 📞 Support and Escalation

### Contact Information

- **Development Team**: dev-team@bitquan.network
- **Operations Team**: ops-team@bitquan.network
- **Security Team**: security@bitquan.network

### Escalation Levels

1. **Level 1**: Basic troubleshooting and monitoring
2. **Level 2**: Advanced debugging and configuration
3. **Level 3**: Infrastructure and security issues
4. **Level 4**: Emergency response and disaster recovery

## 📚 Documentation

- [API Documentation](../docs/api/)
- [Architecture Guide](../docs/architecture/)
- [Security Guidelines](../docs/security/)
- [Troubleshooting Guide](../docs/troubleshooting/)

## 🔄 Version History

- **v1.0.0**: Initial production deployment
- **v1.1.0**: Added monitoring stack
- **v1.2.0**: Enhanced security features
- **v1.3.0**: Auto-scaling and performance optimizations

---

**Note**: This deployment system is designed for production use with high availability, security, and scalability requirements. Always test in staging before deploying to production environments.