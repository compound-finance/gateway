
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

variable "az_secondary" {
  type = string
  description = "AWS availability zone"
  default = "us-east-1c"
}

variable "gateway_private_subnet_cidr" {
  type = string
  default = "10.0.1.0/24"
}

variable "gateway_public_subnet_cidr" {
  type = string
  default = "10.0.2.0/24"
}

variable "gateway_public_secondary_subnet_cidr" {
  type = string
  default = "10.0.3.0/24"
}

variable "node_root_disk_size" {
  type = number
  description = "Disk size to allocate for nodes' root disk in GiB"
  default = 512 # GB
}

variable "authority_node_instance_type" {
  type = string
  description = "Instance ID (AMI) to use for gateway nodes"
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
  default = "ami-02207126df36eb80c" # From https://cloud-images.ubuntu.com/locator/ec2/
}

variable "bastion_instance_type" {
  type = string
  description = "Instance type for bastion node"
  default = "t4g.micro"
}

variable "full_node_instance_type" {
  type = string
  description = "Instance type for full nodes"
  default = "t4g.medium"
}

variable "full_node_count" {
  type = number
  description = "Count of full nodes"
  default = 1
}

variable "full_node_secondary_count" {
  type = number
  description = "Count of full nodes in secondary availability zone"
  default = 1
}

resource "aws_key_pair" "admin_key_pair" {
  key_name   = "admin_key_pair"
  public_key = var.admin_public_key
}

# Create a VPC for our gateway instances
resource "aws_vpc" "gateway_vpc" {
  cidr_block = "10.0.0.0/16"
}

resource "aws_subnet" "gateway_private" {
  availability_zone = var.az
  vpc_id = aws_vpc.gateway_vpc.id
  cidr_block = var.gateway_private_subnet_cidr

  tags = {
    Name = "gateway_private_subnet"
  }
}

resource "aws_subnet" "gateway_public" {
  availability_zone = var.az
  vpc_id = aws_vpc.gateway_vpc.id
  cidr_block = var.gateway_public_subnet_cidr

  tags = {
    Name = "gateway_public_subnet"
  }
}

resource "aws_subnet" "gateway_public_secondary" {
  availability_zone = var.az_secondary
  vpc_id = aws_vpc.gateway_vpc.id
  cidr_block = var.gateway_public_secondary_subnet_cidr

  tags = {
    Name = "gateway_public_secondary_subnet"
  }
}

# Security group restrictions
resource "aws_security_group" "full_node_sg" {
  name        = "full_node_sg"
  description = "Allow traffic ... for now."
  vpc_id      = aws_vpc.gateway_vpc.id

  # Currently, we'll allow all communication
  ingress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "full_node_sg"
  }
}

resource "aws_security_group" "authority_node_sg" {
  name        = "authority_node_sg"
  description = "Allow traffic from public subnet."
  vpc_id      = aws_vpc.gateway_vpc.id

  # Currently, we'll allow all communication
  ingress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "authority_node_sg"
  }
}

resource "aws_security_group" "bastion_node_sg" {
  name        = "bastion_node_sg"
  description = "Allow all traffic."
  vpc_id      = aws_vpc.gateway_vpc.id

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

# Open communication on public subnet
resource "aws_network_acl" "gateway_public_subnet_acl" {
  vpc_id = aws_vpc.gateway_vpc.id
  subnet_ids = [
    aws_subnet.gateway_public.id,
    aws_subnet.gateway_public_secondary.id
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
    Name = "gateway_public_subnet_acl"
  }
}

# Restrict communication on private subnet to only traffic from public
resource "aws_network_acl" "gateway_private_subnet_acl" {
  vpc_id = aws_vpc.gateway_vpc.id
  subnet_ids = [aws_subnet.gateway_private.id]

  # TODO: Consider adding deeper ACL rules
  egress {
    protocol   = "-1"
    rule_no    = 200
    action     = "allow"
    cidr_block = aws_subnet.gateway_public.cidr_block
    from_port  = 0
    to_port    = 0
  }

  egress {
    protocol   = "-1"
    rule_no    = 201
    action     = "allow"
    cidr_block = aws_subnet.gateway_public_secondary.cidr_block
    from_port  = 0
    to_port    = 0
  }

  egress {
    protocol   = "tcp"
    rule_no    = 202
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 80
    to_port    = 80
  }

  egress {
    protocol   = "tcp"
    rule_no    = 203
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 443
    to_port    = 443
  }

  # For git access, can remove once repo is public
  egress {
    protocol   = "tcp"
    rule_no    = 204
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 22
    to_port    = 22
  }

  # For libp2p, open ephemeral ports
  egress {
    protocol   = "tcp"
    rule_no    = 205
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 1024
    to_port    = 65535
  }

  ingress {
    protocol   = "-1"
    rule_no    = 100
    action     = "allow"
    cidr_block = aws_subnet.gateway_public.cidr_block
    from_port  = 0
    to_port    = 0
  }

  ingress {
    protocol   = "-1"
    rule_no    = 101
    action     = "allow"
    cidr_block = aws_subnet.gateway_public_secondary.cidr_block
    from_port  = 0
    to_port    = 0
  }

  ingress {
    protocol   = "tcp"
    rule_no    = 102
    action     = "allow"
    cidr_block = "0.0.0.0/0"
    from_port  = 1024
    to_port    = 65535
  }

  ingress {
    protocol   = "tcp"
    rule_no    = 103
    action     = "allow"
    ipv6_cidr_block = "::/0"
    from_port  = 1024
    to_port    = 65535
  }

  tags = {
    Name = "gateway_private_subnet_acl"
  }
}


module keystore {
  source = "./keystore"

  key_spec = "ECC_SECG_P256K1"
  alias = "alias/eth_notice_signer"
}

resource "aws_instance" "full_node_public" {
  ami                         = var.base_instance_ami
  availability_zone           = var.az
  ebs_optimized               = true
  instance_type               = var.full_node_instance_type
  key_name                    = aws_key_pair.admin_key_pair.key_name
  tenancy                     = var.tenancy
  vpc_security_group_ids      = [aws_security_group.full_node_sg.id]
  subnet_id                   = aws_subnet.gateway_public.id
  associate_public_ip_address = true

  root_block_device {
    volume_size               = var.node_root_disk_size
    delete_on_termination     = false
  }

  count                       = var.full_node_count
}

resource "aws_instance" "full_node_public_secondary" {
  ami                         = var.base_instance_ami
  availability_zone           = var.az_secondary
  ebs_optimized               = true
  instance_type               = var.full_node_instance_type
  key_name                    = aws_key_pair.admin_key_pair.key_name
  tenancy                     = var.tenancy
  vpc_security_group_ids      = [aws_security_group.full_node_sg.id]
  subnet_id                   = aws_subnet.gateway_public_secondary.id
  associate_public_ip_address = true

  root_block_device {
    volume_size               = var.node_root_disk_size
    delete_on_termination     = false
  }

  count                       = var.full_node_secondary_count
}

resource "aws_instance" "authority_node" {
  ami                         = var.base_instance_ami
  availability_zone           = var.az
  ebs_optimized               = true
  instance_type               = var.authority_node_instance_type
  key_name                    = aws_key_pair.admin_key_pair.key_name
  tenancy                     = var.tenancy
  vpc_security_group_ids      = [aws_security_group.authority_node_sg.id]
  subnet_id                   = aws_subnet.gateway_private.id
  associate_public_ip_address = true
  iam_instance_profile        = module.keystore.instance_profile_for_access.name

  root_block_device {
    volume_size               = var.node_root_disk_size
    delete_on_termination     = false
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
  subnet_id                   = aws_subnet.gateway_public.id
  associate_public_ip_address = true
}

resource "aws_eip" "gateway_nat_gw_eip" {
  vpc = true

  tags = {
    Name = "gateway_nat_gw_eip"
  }
}

resource "aws_internet_gateway" "gateway_ig_gw" {
  vpc_id = aws_vpc.gateway_vpc.id
}

resource "aws_nat_gateway" "gateway_nat_gw" {
  allocation_id = aws_eip.gateway_nat_gw_eip.id
  subnet_id     = aws_subnet.gateway_public.id

  depends_on = [aws_internet_gateway.gateway_ig_gw]
}

resource "aws_lb" "full_node_ext_rpc_load_balancer" {
  name                       = "full-node-ext-rpc-load-balancer"
  internal                   = false
  load_balancer_type         = "network"
  drop_invalid_header_fields = true
  subnets                    = [aws_subnet.gateway_public.id, aws_subnet.gateway_public_secondary.id]
  idle_timeout               = 60

  tags = {
    Name = "full_node_ext_rpc_load_balancer"
  }
}

resource "aws_lb_target_group" "full_node_ext_rpc_lb_target_group" {
  name     = "full-node-ext-rpc-lb-tg-rpc"
  port     = 9933
  protocol = "TCP"
  vpc_id   = aws_vpc.gateway_vpc.id

  health_check {
    protocol = "TCP"
    port = 9933
  }
}

resource "aws_lb_listener" "full_node_ext_rpc_lb_listener" {
  load_balancer_arn = aws_lb.full_node_ext_rpc_load_balancer.arn
  port              = 80
  protocol          = "TCP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.full_node_ext_rpc_lb_target_group.arn
  }
}

resource "aws_lb_target_group_attachment" "full_node_ext_rpc_lb_target_group_primary_attachment" {
  count            = length(aws_instance.full_node_public)
  target_group_arn = aws_lb_target_group.full_node_ext_rpc_lb_target_group.arn
  target_id        = aws_instance.full_node_public[count.index].id
  port             = 9933
}

resource "aws_lb_target_group_attachment" "full_node_ext_rpc_lb_target_group_secondary_attachment" {
  count            = length(aws_instance.full_node_public_secondary)
  target_group_arn = aws_lb_target_group.full_node_ext_rpc_lb_target_group.arn
  target_id        = aws_instance.full_node_public_secondary[count.index].id
  port             = 9933
}

resource "aws_lb" "full_node_ext_ws_load_balancer" {
  name                       = "full-node-ext-ws-load-balancer"
  internal                   = false
  load_balancer_type         = "network"
  drop_invalid_header_fields = true
  subnets                    = [aws_subnet.gateway_public.id, aws_subnet.gateway_public_secondary.id]
  idle_timeout               = 60

  tags = {
    Name = "full_node_ext_ws_load_balancer"
  }
}

resource "aws_lb_target_group" "full_node_ext_ws_lb_target_group" {
  name     = "full-node-ext-ws-lb-tg"
  port     = 9944
  protocol = "TCP"
  vpc_id   = aws_vpc.gateway_vpc.id

  health_check {
    protocol = "TCP"
    port = 9944
  }
}

resource "aws_lb_listener" "full_node_ext_ws_lb_listener" {
  load_balancer_arn = aws_lb.full_node_ext_ws_load_balancer.arn
  port              = 80
  protocol          = "TCP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.full_node_ext_ws_lb_target_group.arn
  }
}

resource "aws_lb_target_group_attachment" "full_node_ext_ws_lb_target_group_primary_attachment" {
  count            = length(aws_instance.full_node_public)
  target_group_arn = aws_lb_target_group.full_node_ext_ws_lb_target_group.arn
  target_id        = aws_instance.full_node_public[count.index].id
  port             = 9944
}

resource "aws_lb_target_group_attachment" "full_node_ext_ws_lb_target_group_secondary_attachment" {
  count            = length(aws_instance.full_node_public_secondary)
  target_group_arn = aws_lb_target_group.full_node_ext_ws_lb_target_group.arn
  target_id        = aws_instance.full_node_public_secondary[count.index].id
  port             = 9944
}

resource "aws_lb" "authority_node_ext_gossip_load_balancer" {
  name                       = "authority-node-ext-gossip-lb"
  internal                   = false
  load_balancer_type         = "network"
  drop_invalid_header_fields = true
  subnets                    = [aws_subnet.gateway_public.id, aws_subnet.gateway_public_secondary.id]
  idle_timeout               = 60

  tags = {
    Name = "authority_node_ext_gossip_load_balancer"
  }
}

resource "aws_lb_target_group" "authority_node_ext_gossip_lb_target_group" {
  name     = "authority-node-ext-gossip-lb-tg"
  port     = 30333
  protocol = "TCP"
  vpc_id   = aws_vpc.gateway_vpc.id

  health_check {
    protocol = "TCP"
    port = 30333
  }
}

resource "aws_lb_listener" "authority_node_ext_gossip_lb_listener" {
  load_balancer_arn = aws_lb.authority_node_ext_gossip_load_balancer.arn
  port              = 30333
  protocol          = "TCP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.authority_node_ext_gossip_lb_target_group.arn
  }
}

resource "aws_lb_target_group_attachment" "authority_node_ext_gossip_lb_target_group_attachment" {
  target_group_arn = aws_lb_target_group.authority_node_ext_gossip_lb_target_group.arn
  target_id        = aws_instance.authority_node.id
  port             = 30333
}

resource "aws_route_table" "public_ig_route" {
  vpc_id = aws_vpc.gateway_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.gateway_ig_gw.id
  }

  tags = {
    Name = "public_subnet_ig_route"
  }
}

resource "aws_route_table_association" "public_subnet_ig_route_association" {
  subnet_id      = aws_subnet.gateway_public.id
  route_table_id = aws_route_table.public_ig_route.id
}

resource "aws_route_table_association" "public_secondary_subnet_ig_route_association" {
  subnet_id      = aws_subnet.gateway_public_secondary.id
  route_table_id = aws_route_table.public_ig_route.id
}

resource "aws_route_table" "private_nat_ig_route" {
  vpc_id = aws_vpc.gateway_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.gateway_nat_gw.id
  }

  tags = {
    Name = "private_subnet_nat_ig_route"
  }
}

resource "aws_route_table_association" "private_subnet_nat_ig_route_association" {
  subnet_id      = aws_subnet.gateway_private.id
  route_table_id = aws_route_table.private_nat_ig_route.id
}

output "bastion_ip_address" {
  value = aws_instance.bastion.public_ip
}

output "authority_node_ip_address" {
  value = aws_instance.authority_node.private_ip
}

output "full_node_ip_address" {
  value = aws_instance.full_node_public.*.private_ip
}

output "full_node_secondary_ip_address" {
  value = aws_instance.full_node_public_secondary.*.private_ip
}
