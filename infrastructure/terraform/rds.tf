# Aurora PostgreSQL + RDS Proxy

# KMS Key for RDS
resource "aws_kms_key" "rds" {
  description = "KMS key for RDS encryption"
  deletion_window_in_days = 7

  tags = local.common_tags
}

# Aurora Cluster
resource "aws_rds_cluster" "main" {
  cluster_identifier          = "${var.cluster_name}-aurora"
  engine                      = "aurora-postgresql"
  engine_version              = "15.4"
  database_name               = "erp"
  master_username             = "admin"
  manage_master_user_password = true

  serverlessv2_scaling_configuration {
    min_capacity = 4
    max_capacity = 64
  }

  storage_encrypted       = true
  kms_key_id              = aws_kms_key.rds.arn
  backup_retention_period = 35
  preferred_backup_window  = "03:00-04:00"
  skip_final_snapshot      = false
  final_snapshot_identifier = "${var.cluster_name}-final-snapshot"

  enabled_cloudwatch_logs_exports = ["postgresql"]

  vpc_security_group_ids = [aws_security_group.rds.id]

  db_subnet_group_name = aws_db_subnet_group.main.name

  tags = local.common_tags
}

# Aurora Instance
resource "aws_rds_cluster_instance" "main" {
  count               = 2
  identifier          = "${var.cluster_name}-aurora-instance"
  cluster_identifier  = aws_rds_cluster.main.id
  instance_class      = "db.serverless"
  engine              = aws_rds_cluster.main.engine
  engine_version      = aws_rds_cluster.main.engine_version

  tags = local.common_tags
}

# RDS Proxy for connection pooling
resource "aws_db_proxy" "main" {
  name                   = "${var.cluster_name}-proxy"
  engine_family          = "POSTGRESQL"
  require_tls            = true
  vpc_subnet_ids         = module.vpc.private_subnets
  vpc_security_group_ids = [aws_security_group.rds_proxy.id]

  auth {
    auth_scheme = "SECRETS"
    iam_auth    = "REQUIRED"
    secret_arn  = aws_secretsmanager_secret.db_credentials.arn
  }

  tags = local.common_tags
}

resource "aws_db_proxy_target" "main" {
  db_proxy_name       = aws_db_proxy.main.name
  target_group_name   = "default"
  db_cluster_identifier = aws_rds_cluster.main.id
}

# DB Subnet Group
resource "aws_db_subnet_group" "main" {
  name       = "${var.cluster_name}-db"
  subnet_ids = module.vpc.database_subnets

  tags = local.common_tags
}

# Security Groups
resource "aws_security_group" "rds" {
  name        = "${var.cluster_name}-rds-sg"
  description = "Security group for Aurora RDS"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [module.eks.node_security_group_id]
  }

  tags = local.common_tags
}

resource "aws_security_group" "rds_proxy" {
  name        = "${var.cluster_name}-rds-proxy-sg"
  description = "Security group for RDS Proxy"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [module.eks.node_security_group_id]
  }

  egress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    security_groups = [aws_security_group.rds.id]
  }

  tags = local.common_tags
}

# Secrets Manager for DB credentials
resource "aws_secretsmanager_secret" "db_credentials" {
  name = "${var.cluster_name}-db-credentials"
  recovery_window_in_days = 7

  tags = local.common_tags
}
