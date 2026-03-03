# EKS Cluster Configuration

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "19.15.0"

  cluster_name    = var.cluster_name
  cluster_version = var.cluster_version

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  # OIDC Identity Provider (for IRSA)
  enable_irsa = true

  # Cluster encryption
  cluster_encryption_config = {
    provider_key_arn = aws_kms_key.eks.arn
    resources        = ["secrets"]
  }

  # Cluster endpoint access control (security)
  cluster_endpoint_public_access  = false
  cluster_endpoint_private_access = true

  # Control plane logging
  enabled_cluster_log_types = ["api", "audit", "authenticator", "controllerManager", "scheduler"]

  # Managed Node Groups
  eks_managed_node_groups = {
    system = {
      name           = "system"
      min_size       = 2
      max_size       = 5
      desired_size   = 3
      instance_types = ["m7i.xlarge"]
      capacity_type  = "ON_DEMAND"

      labels = {
        workload-type = "system"
      }

      taints = {
        system = {
          key    = "system"
          value  = "true"
          effect = "NO_SCHEDULE"
        }
      }
    }

    tenants = {
      name           = "tenants"
      min_size       = 3
      max_size       = 50
      desired_size   = 5
      instance_types = ["m7i.xlarge", "m7i.2xlarge"]
      capacity_type  = "MIXED"

      labels = {
        workload-type = "tenant"
      }
    }
  }

  # Cluster add-ons
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
    secrets-store-csi-driver = {
      most_recent = true
    }
  }

  tags = local.common_tags
}

# KMS Key for EKS
resource "aws_kms_key" "eks" {
  description = "KMS key for EKS cluster encryption"
  deletion_window_in_days = 7

  tags = local.common_tags
}
