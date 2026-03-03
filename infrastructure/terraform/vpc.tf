# VPC Configuration

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.0.0"

  name = "${var.cluster_name}-vpc"
  cidr = "10.0.0.0/16"

  azs = ["us-east-1a", "us-east-1b", "us-east-1c"]

  private_subnets = [
    "10.0.32.0/19",
    "10.0.64.0/19",
    "10.0.96.0/19"
  ]

  public_subnets = [
    "10.0.0.0/20",
    "10.0.16.0/20",
    "10.0.48.0/20"
  ]

  database_subnets = [
    "10.0.128.0/24",
    "10.0.144.0/24",
    "10.0.160.0/24"
  ]

  create_database_subnet_group = true

  enable_nat_gateway = true
  single_nat_gateway = true

  enable_vpn_gateway = false

  enable_dns_hostnames = true

  # VPC Flow Logs
  enable_flow_log = true
  flow_log_max_aggregation_interval = 60

  public_subnet_tags = {
    Type = "public"
    "kubernetes.io/role/elb" = "1"
  }

  private_subnet_tags = {
    Type     = "private"
    "kubernetes.io/role/internal-elb" = "1"
  }

  database_subnet_tags = {
    Type = "database"
  }

  tags = local.common_tags
}
