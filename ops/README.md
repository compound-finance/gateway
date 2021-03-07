
# Gateway Operations

## Starting a Gateway Node

This repo includes code to run a Gateway Node on AWS using [Terraform](https://www.terraform.io/) and [Ansible](https://www.ansible.com/). To get started, ensure you have both Terraform and Ansible installed.

You may also deploy Gateway on another provider, Docker, etc. This repo describes a good guideline for a secure deployment-- ensure if you deploy on your own, you properly consider the security of your node.

For more information on the design, see the [Architecture Doc](./ARCHITECTURE.md).

### Building AWS Infra

From the `ops` directory, you should first set-up an AWS account and create a bucket to store your terraform state. Then run:

```sh
terraform init -upgrade \
  -backend-config="bucket=gateway" \
  -backend-config="key=tfstate" \
  -backend-config="region=us-east-1"
```

Note: if you have an error, you may need to [review your AWS credentials](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#authentication) for Terraform.

Also, you will need to create a public & private key pair to access your instance. We assume that's defined in `~/.ssh/id_rsa_gateway.pub`.

Next, plan the terraform changes:

```sh
terraform plan \
  -var admin_public_key="$(cat ~/.ssh/id_rsa_gateway.pub)"
```

Then, if that looks good, apply the terraform changes:

```sh
terraform apply \
  -var admin_public_key="$(cat ~/.ssh/id_rsa_gateway.pub)"
```

Once you have everything up, you'll need to construct your Ansible inventory and `ssh_config`. This can be done by running:

```sh
terraform output -json | ./ansible/generate_inv.py
```

Note: you'll need python3 installed. You may need to run this command differently in Powershell.

Note: after you change any nodes created by terraform, you will need to re-run this command.

### Setting up Gateway application

Setting up each node is a matter of simply running the Ansible playbooks. Make sure your inventory is up-to-date by running the `generate_inv.py` command above!

To run the playbook and configure the servers, run:

```sh
ansible-playbook -i hosts --ssh-extra-args "-F ./ssh_config" ansible/chain.yml
```

Note: while Gateway is private, you will need to add a deploy key to the repo and give that deploy key to the servers, like so:

```sh
env deploy_key="$HOME/.ssh/id_rsa_gateway_deploy" ansible-playbook -i hosts --ssh-extra-args "-F ./ssh_config" ansible/chain.yml
```

### Resyncing and Restarting a Gateway deployment
build another chain spec
```sh
# edit chain_spec.rs
./target/release/gateway build-spec staging > gatewayChainSpec.json
# edit myCustomSpec.json
./target/release/gateway build-spec --chain=gatewayChainSpec.json --raw > gatewayChainSpecRaw.json
```

```sh
# in deployment directory eg brr/charlie
ansible-playbook -i hosts --ssh-extra-args "-F ./ssh_config" purge-and-restart.yml

# or in gateway
```

## Best Practices

If you need to run multiple isolated deployments, the best practice is to create a directory `deployment` and then sub-folders for each isolated deployment with a `main.tf` that references `./tf/main.tf`'s module. This is the official terraform way to handle fully hermetic deployments (i.e. more isolated than terraform workspaces). Note: the `deployment` folder is git ignored.

## Contributing

Please create an issue or pull request. Note: the goal here is to build a golden path fully-secured deployment. Thus, we may not add frivilous features. If you need more features, consider using the isolated deployments pattern above and adding to the `main.tf` you create.
