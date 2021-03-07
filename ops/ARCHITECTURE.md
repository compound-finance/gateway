
## Gateway Node Architecture

This doc describes the architecture for Gateway that is deployed via Terraform and Ansible.

### Network

We deploy a VPC with three sub-nets (`private`, `public` and `public-backup`). Note: `public-backup` is a requirement for Amazon Application Load Balancers, since they require two subnets to route to. As expected, the `public` sub-nets are accessible to the external Internet via an Internet Gateway. The `private` sub-net is inaccessible to the greater Internet and all requests must be routed through a VPN node in the `public` subnet. We sync full nodes in the `public` sub-nets which peer with the authority node in the `private` subnet. RPC and ws requests will only go to the full nodes to prevent DoS or other attacks to the authority node itself. Additionally, ssh access to any `private` node must be routed through a bastion node in the `public` subnet.

This set-up generally ensures safety as the core node (i.e. the Gateway Authority Node) is heavily firewalled (via network security, security groups, network ACLs and firewalls). We are currently routing RPC and websocket requests directly to the Authority Node, but we will likely replace this with a Full Node which itself is peered to the Authority Node to add an additional layer of protection (e.g. against DoS attacks).

#### Network TODOs

- [ ]: Tighten the network ACLs as an additional layer of security
- [ ]: Add firewalls, too.
- [ ]: Consider inter-Gateway Node communication
- [X]: Add "Full Nodes" to picture
- [ ]: Add KMS

### Instance Configuration

All nodes currently run ubuntu by default. We generally use `systemd` for daemon supervision (e.g. to run the gateway binary). Software and configuration are synced via Ansible tasks, where the hardware for the nodes is managed by Terraform. Several nodes run hosted software, such as the VPN node (to allow external communication from the private subnet) and the Ethereum node (to sync the current state of Ethereum) are managed by AWS directly. For security reasons, we generally run as many components on our own dedicated hardware as possible.

#### Authority Node

The Authority Node builds and runs a release of the Gateway core binary. The binary is run via `systemd`. Communication is currently routed directly to this instance via the application load balancer.

#### Full Nodes

We run several full nodes in public sub-nets. These nodes peer with our full node and can be used by external clients to query the state of the system. These nodes are behind a load balancer and a CloudFlare proxy.
