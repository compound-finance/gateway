
# Backend
terraform {
  backend "s3" {}
}

# Configuration for AWS
provider "aws" {
  region = var.region
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

variable "gatew_public_subnet_cidr" {
  type = string
  default = "10.0.2.0/24"
}

variable "gatew_public_secondary_subnet_cidr" {
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

module tf {
  source = "./tf"

  region = var.region
  az = var.az
  az_secondary = var.az_secondary
  gatew_private_subnet_cidr = var.gatew_private_subnet_cidr
  gatew_public_subnet_cidr = var.gatew_public_subnet_cidr
  gatew_public_secondary_subnet_cidr = var.gatew_public_secondary_subnet_cidr
  authority_node_instance_type = var.authority_node_instance_type
  node_root_disk_size = var.node_root_disk_size
  tenancy = var.tenancy
  admin_public_key = var.admin_public_key
  base_instance_ami = var.base_instance_ami
  bastion_instance_type = var.bastion_instance_type
  full_node_instance_type = var.full_node_instance_type
  full_node_count = var.full_node_count
  full_node_secondary_count = var.full_node_secondary_count
}

output "bastion_ip_address" {
  value = module.tf.bastion_ip_address
}

output "authority_node_ip_address" {
  value = module.tf.authority_node_ip_address
}

output "full_node_ip_address" {
  value = module.tf.full_node_ip_address
}

output "full_node_secondary_ip_address" {
  value = module.tf.full_node_secondary_ip_address
}
