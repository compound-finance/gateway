
# Compound Chain Operations

## Starting a Compound Chain Node

This repo includes code to run a Compound Chain Node on AWS using [Terraform](https://www.terraform.io/) and [Ansible](https://www.ansible.com/). To get started, ensure you have both Terraform and Ansible installed.

You may also deploy Compound Chain on another provider, Docker, etc. This repo describes a good guideline for a secure deployment-- ensure if you deploy on your own, you properly consider the security of your node.

For more information on the design, see the [Architecture Doc](./ARCHITECTURE.md).

### Building AWS Infra

From the `ops` directory, you should first set-up an AWS account and create a bucket to store your terraform state. Then run:

```sh
AWS_PROFILE=compound-dev-1 terraform init -upgrade \
  -backend-config="bucket=compound-chain" \
  -backend-config="key=tfstate" \
  -backend-config="region=us-east-1"
```

Note: if you have an error, you may need to [review your AWS credentials](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#authentication) for Terraform.

Also, you will need to create a public & private key pair to access your instance. We assume that's defined in `~/.ssh/id_rsa_compound_chain.pub`.

Next, plan the terraform changes:

```sh
AWS_PROFILE=compound-dev-1 terraform plan \
  -var admin_public_key="$(cat ~/.ssh/id_rsa_compound_chain.pub)"
```

Then, if that looks good, apply the terraform changes:

```sh
AWS_PROFILE=compound-dev-1 terraform apply \
  -var admin_public_key="$(cat ~/.ssh/id_rsa_compound_chain.pub)"
```

Once you have everything up, you'll need to construct your Ansible inventory and `ssh_config`. This can be done by running:

```sh
AWS_PROFILE=compound-dev-1 terraform output -json | ./generate_inv.py
```

Note: you'll need python3 installed. You may need to run this command differently in Powershell.

Note: after you change any nodes created by terraform, you will need to re-run this command.

### Setting up Compound Chain application

Lorem ipsum

TODO: Set-up `ssh_config`

First, it's important to set-up basic build essentials on your node (e.g. rust).

```sh
ansible-playbook -v tasks/build-essentials.yml
```

Next, you'll need to build the Compound Chain from source. Note: you'll need a deploy key to sync to the node to pull in the source while the project is still not publically available.

```sh
ansible-playbook -v tasks/compound-chain.yml -e "deploy_key=$(cd ~/.ssh && pwd)/id_rsa_compound_chain_deploy"
```

Finally, you need to start the Compound Chain service to begin the chain.

```sh
ansible-playbook -v tasks/compound-service.yml
```
