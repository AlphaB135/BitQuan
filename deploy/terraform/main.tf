terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.20"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.10"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.5"
    }
  }
  
  backend "s3" {
    bucket = "bitquan-terraform-state"
    key    = "bitquan-infrastructure/terraform.tfstate"
    region = "us-west-2"
    encrypt = true
    dynamodb_table = "bitquan-terraform-locks"
  }
}

provider "aws" {
  region = var.aws_region
  
  default_tags {
    tags = {
      Project     = "BitQuan"
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)
  token                  = data.aws_eks_cluster_auth.cluster.token
}

provider "helm" {
  kubernetes {
    host                   = module.eks.cluster_endpoint
    cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)
    token                  = data.aws_eks_cluster_auth.cluster.token
  }
}

# Variables
variable "environment" {
  description = "Deployment environment"
  type        = string
}

variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-west-2"
}

variable "cluster_version" {
  description = "EKS cluster version"
  type        = string
  default     = "1.28"
}

variable "node_count" {
  description = "Number of worker nodes"
  type        = number
  default     = 3
}

variable "instance_type" {
  description = "EC2 instance type for worker nodes"
  type        = string
  default     = "m5.xlarge"
}

# Local values
locals {
  name_prefix = "${var.environment}-bitquan"
  common_tags = {
    Environment = var.environment
    Project     = "BitQuan"
    ManagedBy   = "Terraform"
  }
}

# Random resources
resource "random_pet" "this" {
  length = 2
}

# VPC
module "vpc" {
  source = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = local.name_prefix
  cidr = "10.0.0.0/16"

  azs             = ["${var.aws_region}a", "${var.aws_region}b", "${var.aws_region}c"]
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]

  enable_nat_gateway = true
  enable_vpn_gateway = false
  enable_dns_hostnames = true
  enable_dns_support = true

  tags = local.common_tags
}

# EKS Cluster
module "eks" {
  source = "terraform-aws-modules/eks/aws"
  version = "~> 19.0"

  cluster_name    = local.name_prefix
  cluster_version = var.cluster_version
  vpc_id          = module.vpc.vpc_id
  subnet_ids      = module.vpc.private_subnets

  cluster_endpoint_public_access = true
  cluster_endpoint_private_access = true

  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
    }
    aws-ebs-csi-driver = {
      most_recent = true
    }
  }

  eks_managed_node_groups = {
    bitquan_nodes = {
      desired_size = var.node_count
      max_size     = var.node_count + 2
      min_size     = var.node_count - 1

      instance_types = [var.instance_type]
      capacity_type  = "ON_DEMAND"

      k8s_labels = {
        Environment = var.environment
        Project     = "BitQuan"
        NodeGroup   = "bitquan-nodes"
      }

      update_config = {
        max_unavailable_percentage = 33
      }

      iam_role_additional_policies = {
        AmazonEC2ContainerRegistryReadOnly = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
        CloudWatchAgentServerPolicy        = "arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy"
      }
    }
  }

  tags = local.common_tags
}

# EKS Cluster Auth
data "aws_eks_cluster_auth" "cluster" {
  name = module.eks.cluster_name
}

# IAM Role for BitQuan Service Account
resource "aws_iam_policy" "bitquan_policy" {
  name        = "${local.name_prefix}-bitquan-policy"
  description = "Policy for BitQuan nodes"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:ListBucket"
        ]
        Resource = [
          "arn:aws:s3:::bitquan-${var.environment}-*",
          "arn:aws:s3:::bitquan-${var.environment}-*/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "dynamodb:GetItem",
          "dynamodb:PutItem",
          "dynamodb:UpdateItem",
          "dynamodb:DeleteItem",
          "dynamodb:Query",
          "dynamodb:Scan"
        ]
        Resource = "arn:aws:dynamodb:${var.aws_region}:*:table/bitquan-${var.environment}-*"
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:${var.aws_region}:*:log-group:/aws/eks/bitquan-${var.environment}-*"
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "bitquan_attach" {
  policy_arn = aws_iam_policy.bitquan_policy.arn
  role       = module.eks.iam_role_name
}

# Storage
resource "aws_ebs_volume" "bitquan_data" {
  count             = var.node_count
  availability_zone = element(module.vpc.azs, count.index)
  size              = 100
  type              = "gp3"
  encrypted         = true

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-data-${count.index}"
  })
}

# S3 Bucket for blockchain data
resource "aws_s3_bucket" "bitquan_blockchain" {
  bucket = "bitquan-${var.environment}-blockchain-${random_pet.this.id}"
}

resource "aws_s3_bucket_versioning" "bitquan_blockchain" {
  bucket = aws_s3_bucket.bitquan_blockchain.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_encryption" "bitquan_blockchain" {
  bucket = aws_s3_bucket.bitquan_blockchain.id

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}

resource "aws_s3_bucket_public_access_block" "bitquan_blockchain" {
  bucket = aws_s3_bucket.bitquan_blockchain.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# DynamoDB for metadata
resource "aws_dynamodb_table" "bitquan_metadata" {
  name           = "bitquan-${var.environment}-metadata"
  billing_mode   = "PAY_PER_REQUEST"
  hash_key       = "node_id"

  attribute {
    name = "node_id"
    type = "S"
  }

  attribute {
    name = "timestamp"
    type = "N"
  }

  global_secondary_index {
    name     = "timestamp_index"
    hash_key = "node_id"
    range_key = "timestamp"
  }

  tags = local.common_tags
}

# CloudWatch Log Group
resource "aws_cloudwatch_log_group" "bitquan_logs" {
  name              = "/aws/eks/bitquan-${var.environment}/nodes"
  retention_in_days = 30

  tags = local.common_tags
}

# Monitoring Stack (Helm)
resource "helm_release" "prometheus" {
  name       = "prometheus"
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "kube-prometheus-stack"
  namespace  = "monitoring"
  create_namespace = true

  set {
    name  = "prometheus.prometheusSpec.storageSpec.volumeClaimTemplate.spec.resources.requests.storage"
    value = "50Gi"
  }

  set {
    name  = "grafana.adminPassword"
    value = var.environment == "mainnet" ? "changeme!" : "admin123"
  }

  depends_on = [module.eks]
}

# BitQuan Namespace
resource "kubernetes_namespace" "bitquan" {
  metadata {
    name = "bitquan-${var.environment}"
    labels = {
      Environment = var.environment
      Project     = "BitQuan"
    }
  }
}

# ConfigMap for BitQuan configuration
resource "kubernetes_config_map" "bitquan_config" {
  metadata {
    name      = "bitquan-config"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
  }

  data = {
    "config.toml" = templatefile("${path.root}/../../config/${var.environment}.toml", {
      environment = var.environment
      aws_region = var.aws_region
      s3_bucket  = aws_s3_bucket.bitquan_blockchain.bucket
      dynamodb_table = aws_dynamodb_table.bitquan_metadata.name
    })
  }
}

# Secret for sensitive data
resource "kubernetes_secret" "bitquan_secrets" {
  metadata {
    name      = "bitquan-secrets"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
  }

  data = {
    "jwt-secret" = base64encode(random_password.jwt_secret.result)
  }
}

resource "random_password" "jwt_secret" {
  length  = 32
  special = false
}

# BitQuan Deployment
resource "kubernetes_deployment" "bitquan_node" {
  metadata {
    name      = "bitquan-node"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
    labels = {
      app = "bitquan-node"
      environment = var.environment
    }
  }

  spec {
    replicas = var.node_count

    selector {
      match_labels = {
        app = "bitquan-node"
      }
    }

    template {
      metadata {
        labels = {
          app = "bitquan-node"
          environment = var.environment
        }
      }

      spec {
        service_account_name = "bitquan-node"

        container {
          name  = "bitquan-node"
          image = "ghcr.io/${var.github_repository}/bitquan-node:latest"
          image_pull_policy = "Always"

          port {
            container_port = 8080
            name          = "http"
          }

          port {
            container_port = 8081
            name          = "metrics"
          }

          port {
            container_port = 3333
            name          = "stratum"
          }

          env {
            name  = "RUST_LOG"
            value = "info,bitquan=debug"
          }

          env {
            name = "CONFIG_PATH"
            value_from {
              config_map_key_ref {
                name = kubernetes_config_map.bitquan_config.metadata[0].name
                key  = "config.toml"
              }
            }
          }

          env {
            name = "JWT_SECRET"
            value_from {
              secret_key_ref {
                name = kubernetes_secret.bitquan_secrets.metadata[0].name
                key  = "jwt-secret"
              }
            }
          }

          resources {
            limits = {
              cpu    = "2000m"
              memory = "4Gi"
            }
            requests = {
              cpu    = "1000m"
              memory = "2Gi"
            }
          }

          liveness_probe {
            http_get {
              path = "/health"
              port = 8080
            }
            initial_delay_seconds = 30
            period_seconds        = 10
            timeout_seconds       = 5
            failure_threshold     = 3
          }

          readiness_probe {
            http_get {
              path = "/health"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 5
            timeout_seconds       = 3
            failure_threshold     = 3
          }

          volume_mount {
            name       = "data"
            mount_path = "/data"
          }
        }

        volume {
          name = "data"
          persistent_volume_claim {
            claim_name = kubernetes_persistent_volume_claim.bitquan_data.metadata[0].name
          }
        }
      }
    }
  }
}

# PVC for data storage
resource "kubernetes_persistent_volume_claim" "bitquan_data" {
  metadata {
    name      = "bitquan-data"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
  }

  spec {
    access_modes = ["ReadWriteOnce"]
    resources {
      requests = {
        storage = "100Gi"
      }
    }
    storage_class_name = "gp3"
  }
}

# Service
resource "kubernetes_service" "bitquan_node" {
  metadata {
    name      = "bitquan-node"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
    labels = {
      app = "bitquan-node"
    }
  }

  spec {
    selector = {
      app = "bitquan-node"
    }

    port {
      name        = "http"
      port        = 80
      target_port = 8080
    }

    port {
      name        = "metrics"
      port        = 81
      target_port = 8081
    }

    port {
      name        = "stratum"
      port        = 3333
      target_port = 3333
    }

    type = "LoadBalancer"
  }
}

# Ingress for mainnet
resource "kubernetes_ingress" "bitquan_mainnet" {
  count = var.environment == "mainnet" ? 1 : 0
  metadata {
    name      = "bitquan-mainnet"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
    annotations = {
      "kubernetes.io/ingress.class" = "nginx"
      "cert-manager.io/cluster-issuer" = "letsencrypt-prod"
      "nginx.ingress.kubernetes.io/rate-limit" = "100"
    }
  }

  spec {
    tls {
      hosts       = ["mainnet.bitquan.network"]
      secret_name = "bitquan-mainnet-tls"
    }

    rule {
      host = "mainnet.bitquan.network"
      http {
        path {
          path = "/"
          backend {
            service {
              name = kubernetes_service.bitquan_node.metadata[0].name
              port {
                number = 80
              }
            }
          }
        }
      }
    }
  }
}

# Horizontal Pod Autoscaler
resource "kubernetes_horizontal_pod_autoscaler" "bitquan_node" {
  metadata {
    name      = "bitquan-node"
    namespace = kubernetes_namespace.bitquan.metadata[0].name
  }

  spec {
    scale_target_ref {
      api_version = "apps/v1"
      kind       = "Deployment"
      name       = kubernetes_deployment.bitquan_node.metadata[0].name
    }

    min_replicas = var.node_count
    max_replicas = var.node_count * 2

    metric {
      type = "Resource"
      resource {
        name = "cpu"
        target {
          type               = "Utilization"
          average_utilization = 70
        }
      }
    }

    metric {
      type = "Resource"
      resource {
        name = "memory"
        target {
          type               = "Utilization"
          average_utilization = 80
        }
      }
    }
  }
}

# Outputs
output "cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "s3_bucket" {
  description = "S3 bucket for blockchain data"
  value       = aws_s3_bucket.bitquan_blockchain.bucket
}

output "dynamodb_table" {
  description = "DynamoDB table for metadata"
  value       = aws_dynamodb_table.bitquan_metadata.name
}

output "load_balancer_url" {
  description = "Load balancer URL"
  value       = kubernetes_service.bitquan_node.status.0.load_balancer.0.ingress.0.hostname
}