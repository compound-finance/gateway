
variable "region" {
  type = string
  description = "AWS region"
  default = "us-east-1"
}

variable "az" {
  type = string
  description = "AWS availability zone"
  default = "us-east-1a"
}

variable "az_backup" {
  type = string
  description = "AWS availability zone"
  default = "us-east-1c"
}

variable "authority_node_private_subnet_cidr" {
  type = string
  default = "10.0.1.0/24"
}

variable "authority_node_public_subnet_cidr" {
  type = string
  default = "10.0.2.0/24"
}

variable "authority_node_public_backup_subnet_cidr" {
  type = string
  default = "10.0.3.0/24"
}

variable "authority_node_disk_size" {
  type = number
  description = "Disk size to allocate for authority node in GB"
  default = 1024 # 1 TB
}

variable "authority_node_instance_type" {
  type = string
  description = "Instance ID (AMI) to use for authority node"
  default = "m6g.large" # TODO: Choose best default instance type
}

variable "tenancy" {
  type = string
  description = "Tenacy: default, dedicated or host"
  default = "default"
}

variable "admin_public_key" {
  type = string
}

variable "base_instance_ami" {
  type = string
  description = "AWS ami image to use for core instances"
  default = "ami-05ca751716e10fe16" # See https://wiki.centos.org/Cloud/AWS#Official_and_current_CentOS_Public_Images
}

variable "bastion_instance_type" {
  type = string
  description = "Instance type for bastion node"
  default = "t4g.micro"
}

# AWS Provider
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 3.0"
    }
  }
}

# Configuration for AWS
provider "aws" {
  region = var.region
}

resource "aws_key_pair" "admin_key_pair" {
  key_name   = "admin_key_pair"
  public_key = var.admin_public_key
}

# Create a VPC for our authority node instance
resource "aws_vpc" "authority_node_vpc" {
  cidr_block = "10.0.0.0/16"
}

resource "aws_subnet" "authority_node_private" {
  availability_zone = var.az
  vpc_id = aws_vpc.authority_node_vpc.id
  cidr_block = var.authority_node_private_subnet_cidr

  tags = {
    Name = "authority_node_private_subnet"
  }
}

resource "aws_subnet" "authority_node_public" {
  availability_zone = var.az
  vpc_id = aws_vpc.authority_node_vpc.id
  cidr_block = var.authority_node_public_subnet_cidr

  tags = {
    Name = "authority_node_public_subnet"
  }
}

resource "aws_subnet" "authority_node_public_backup" {
  availability_zone = var.az_backup
  vpc_id = aws_vpc.authority_node_vpc.id
  cidr_block = var.authority_node_public_backup_subnet_cidr

  tags = {
    Name = "authority_node_public_backup_subnet"
  }
}

# Security group restrictions
resource "aws_security_group" "authority_node_sg" {
  name        = "authority_node_sg"
  description = "Allow traffic from public subnet."
  vpc_id      = aws_vpc.authority_node_vpc.id

  # Currently, we'll allow communication from our public subnet only
  ingress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = [
      aws_subnet.authority_node_public.cidr_block,
      aws_subnet.authority_node_public_backup.cidr_block
    ]
  }
  
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = [
      aws_subnet.authority_node_public.cidr_block,
      aws_subnet.authority_node_public_backup.cidr_block
    ]
  }

  tags = {
    Name = "authority_node_sg"
  }
}

resource "aws_security_group" "bastion_node_sg" {
  name        = "bastion_node_sg"
  description = "Allow all traffic."
  vpc_id      = aws_vpc.authority_node_vpc.id

  ingress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    ipv6_cidr_blocks = ["::/0"]
  }
  
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    ipv6_cidr_blocks = ["::/0"]
  }

  tags = {
    Name = "bastion_node_sg"
  }
}

# Security group restrictions
resource "aws_security_group" "authority_node_lb_sg" {
  name        = "authority_node_lb_sg"
  description = "Allow gossip, rpc and ws traffic in. All traffic out."
  vpc_id      = aws_vpc.authority_node_vpc.id

  # TODO: Consider securing this
  ingress {
    description = "Gossip port"
    from_port   = 30333
    to_port     = 30333
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # TODO: Consider securing this
  ingress {
    description = "RPC port"
    from_port   = 9933
    to_port     = 9933
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # TODO: Consider securing this
  ingress {
    description = "WS port"
    from_port   = 9944
    to_port     = 9944
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # TODO: Secure this, as well.
  egress {
    from_port   = 0
    to_port     = 65535
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "authority_node_lb_sg"
  }
}

# Open communication on public subnet
resource "aws_network_acl" "authority_node_public_acl" {
  vpc_id = aws_vpc.authority_node_vpc.id
  subnet_ids = [
    aws_subnet.authority_node_public.id,
    aws_subnet.authority_node_public_backup.id
  ]

  # TODO: Consider adding deeper ACL rules
  egress {
    protocol   = "-1"
    rule_no    = 200
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 0
    to_port    = 0
  }

  egress {
    protocol   = "-1"
    rule_no    = 201
    action     = "allow"
    ipv6_cidr_block = "::/0"
    from_port  = 0
    to_port    = 0
  }

  ingress {
    protocol   = "-1"
    rule_no    = 100
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 0
    to_port    = 0
  }

  ingress {
    protocol   = "-1"
    rule_no    = 101
    action     = "allow"
    ipv6_cidr_block = "::/0"
    from_port  = 0
    to_port    = 0
  }

  tags = {
    Name = "authority_node_public_acl"
  }
}

# Restrict communication on private subnet to only traffic from public
resource "aws_network_acl" "authority_node_private_acl" {
  vpc_id = aws_vpc.authority_node_vpc.id
  subnet_ids = [aws_subnet.authority_node_private.id]

  # TODO: Consider adding deeper ACL rules
  egress {
    protocol   = "-1"
    rule_no    = 200
    action     = "allow"
    cidr_block = aws_subnet.authority_node_public.cidr_block
    from_port  = 0
    to_port    = 0
  }

  egress {
    protocol   = "-1"
    rule_no    = 201
    action     = "allow"
    cidr_block = aws_subnet.authority_node_public_backup.cidr_block
    from_port  = 0
    to_port    = 0
  }

  ingress {
    protocol   = "-1"
    rule_no    = 100
    action     = "allow"
    cidr_block = aws_subnet.authority_node_public.cidr_block
    from_port  = 0
    to_port    = 0
  }

  ingress {
    protocol   = "-1"
    rule_no    = 101
    action     = "allow"
    cidr_block = aws_subnet.authority_node_public_backup.cidr_block
    from_port  = 0
    to_port    = 0
  }

  tags = {
    Name = "authority_node_public_acl"
  }
}

resource "aws_ebs_volume" "authority_node_volume" {
  availability_zone = var.az
  size              = var.authority_node_disk_size
  type              = "gp2"
  # encrypted?

  tags = {
    Name = "authority_node_volume"
  }
}

resource "aws_instance" "authority_node" {
  ami                         = var.base_instance_ami
  availability_zone           = var.az
  ebs_optimized               = true
  instance_type               = var.authority_node_instance_type
  key_name                    = aws_key_pair.admin_key_pair.key_name
  tenancy                     = var.tenancy
  vpc_security_group_ids      = [aws_security_group.authority_node_sg.id]
  subnet_id                   = aws_subnet.authority_node_private.id
  associate_public_ip_address = false

  metadata_options {
    http_endpoint = "disabled"
  }
}

resource "aws_instance" "bastion" {
  ami                         = var.base_instance_ami
  availability_zone           = var.az
  ebs_optimized               = true
  instance_type               = var.bastion_instance_type
  key_name                    = aws_key_pair.admin_key_pair.key_name
  tenancy                     = var.tenancy # Same tenacy as authority node?
  vpc_security_group_ids      = [aws_security_group.bastion_node_sg.id]
  subnet_id                   = aws_subnet.authority_node_public.id
  associate_public_ip_address = true

  metadata_options {
    http_endpoint = "disabled"
  }
}

resource "aws_eip" "authority_node_nat_gw_eip" {
  vpc = true

  tags = {
    Name = "authority_node_nat_gw_eip"
  }
}

resource "aws_internet_gateway" "authority_node_ig_gw" {
  vpc_id = aws_vpc.authority_node_vpc.id
}

resource "aws_nat_gateway" "authority_node_nat_gw_eip" {
  allocation_id = aws_eip.authority_node_nat_gw_eip.id
  subnet_id     = aws_subnet.authority_node_public.id

  depends_on = [aws_internet_gateway.authority_node_ig_gw]
}

resource "aws_lb" "authority_node_load_balancer" {
  name                       = "authority-node-load-balancer"
  internal                   = false
  load_balancer_type         = "application"
  security_groups            = [aws_security_group.authority_node_lb_sg.id]
  drop_invalid_header_fields = true
  subnets                    = [aws_subnet.authority_node_public.id, aws_subnet.authority_node_public_backup.id]
  idle_timeout               = 60

  # TODO: Add access logs?
  # access_logs {
  #   bucket  = aws_s3_bucket.lb_logs.bucket
  #   prefix  = "test-lb"
  #   enabled = true
  # }

  tags = {
    Name = "authority_node_load_balancer"
  }
}

resource "aws_lb_target_group" "authority_node_target_group_rpc" {
  name     = "authority-node-tg-rpc"
  port     = 9933
  protocol = "HTTP"
  vpc_id   = aws_vpc.authority_node_vpc.id
}

resource "aws_lb_listener" "authority_node_lb_listener_rpc" {
  load_balancer_arn = aws_lb.authority_node_load_balancer.arn
  port              = 9933
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.authority_node_target_group_rpc.arn
  }
}

resource "aws_lb_target_group_attachment" "authority_node_lb_target_group_attachment_rpc" {
  target_group_arn = aws_lb_target_group.authority_node_target_group_rpc.arn
  target_id        = aws_instance.authority_node.id
  port             = 9933
}

resource "aws_lb_target_group" "authority_node_target_group_ws" {
  name     = "authority-node-tg-ws"
  port     = 9944
  protocol = "HTTP"
  vpc_id   = aws_vpc.authority_node_vpc.id
}

resource "aws_lb_listener" "authority_node_lb_listener_ws" {
  load_balancer_arn = aws_lb.authority_node_load_balancer.arn
  port              = 9944
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.authority_node_target_group_ws.arn
  }
}

resource "aws_lb_target_group_attachment" "authority_node_lb_target_group_attachment_ws" {
  target_group_arn = aws_lb_target_group.authority_node_target_group_ws.arn
  target_id        = aws_instance.authority_node.id
  port             = 9944
}

resource "aws_lb_target_group" "authority_node_target_group_gossip" {
  name     = "authority-node-tg-gossip"
  port     = 30333
  protocol = "HTTP"
  vpc_id   = aws_vpc.authority_node_vpc.id
}

resource "aws_lb_listener" "authority_node_lb_listener_gossip" {
  load_balancer_arn = aws_lb.authority_node_load_balancer.arn
  port              = 30333
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.authority_node_target_group_gossip.arn
  }
}

resource "aws_lb_target_group_attachment" "authority_node_lb_target_group_attachment_gossip" {
  target_group_arn = aws_lb_target_group.authority_node_target_group_gossip.arn
  target_id        = aws_instance.authority_node.id
  port             = 30333
}

resource "aws_route_table" "authority_node_public_subnet_ig_route" {
  vpc_id = aws_vpc.authority_node_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.authority_node_ig_gw.id
  }

  tags = {
    Name = "authority_node_public_subnet_ig_route"
  }
}

resource "aws_route_table_association" "authority_node_public_subnet_ig_route_association" {
  subnet_id      = aws_subnet.authority_node_public.id
  route_table_id = aws_route_table.authority_node_public_subnet_ig_route.id
}
