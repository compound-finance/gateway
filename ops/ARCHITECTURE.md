
## Compound Chain Node Architecture

This doc describes the architecture for Compound Chain that is deployed via Terraform and Ansible.

### Network

We deploy a VPC with three sub-nets (`private`, `public` and `public-backup`). Note: `public-backup` is a requirement for Amazon Application Load Balancers, since they require two subnets to route to. As expected, the `public` sub-nets are accessible to the external Internet via an Internet Gateway. The `private` sub-net is inaccessible to the greater Internet and all requests must be routed through a VPN node in the `public` subnet. Inbound requests to the `private` subnet generally go through an Application Load Balancer. Additionally, ssh access to `private` nodes are through a bastion node.

This set-up generally ensures safety as the core node (i.e. the Compound Chain Authority Node) is heavily firewalled (via network security, security groups, network ACLs and firewalls).

### TODOs

- [ ]: Tighten the network ACLs as an additional layer of security
- [ ]: Add firewalls, too.
- [ ]: Consider inter-Compound Chain Node communication
- [ ]: Add "Full Nodes" to picture
- [ ]: Add KMS
