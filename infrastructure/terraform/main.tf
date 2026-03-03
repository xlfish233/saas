# ERP SaaS Infrastructure - Main Configuration

terraform {
  required_version = ">= 1.5.0"
  
  backend "s3" {
    bucket = "erp-saas-terraform-state"
    key    = "terraform.tfstate"
    region = "us-east-1"
    encrypt = true
  }
}

provider "aws" {
  region = var.aws_region
}

variable "aws_region" {
  description = "AWS Region"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "production"
}

variable "cluster_name" {
  description = "EKS Cluster name"
  type        = string
  default     = "erp-saas-cluster"
}

variable "cluster_version" {
  description = "EKS Cluster version"
  type        = string
  default     = "1.28"
}

# Common tags
locals {
  common_tags = {
    Environment = var.environment
    Project     = "erp-saas"
    ManagedBy   = "terraform"
  }
}
