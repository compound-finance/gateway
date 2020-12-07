
# Compound Chain Operations

## Starting a Compound Chain Node

This repo includes code to run a Compound Chain Node on AWS using [Terraform](https://www.terraform.io/) and [Ansible](https://www.ansible.com/). To get started, ensure you have both Terraform and Ansible installed.

You may also deploy Compound Chain on another provider, Docker, etc. This repo describes a good guideline for a secure deployment-- ensure if you deploy on your own, you properly consider the security of your node.

For more information on the design, see the [Architecture Doc](./ARCHITECTURE.md).

### Building AWS Infra

From the `ops` directory, run:

```sh
terraform init -upgrade tf 
```

Note: if you have an error, you may need to [review your AWS credentials](https://registry.terraform.io/providers/hashicorp/aws/latest/docs#authentication) for Terraform.

Also, you will need to create a public & private key pair to access your instance. We assume that's defined in `~/.ssh/id_rsa_compound_chain.pub`.

Next, plan the terraform changes:

```sh
 terraform plan -var admin_public_key="$(cat ~/.ssh/id_rsa_compound_chain.pub)" tf
```

Then, if that looks good, apply the terraform changes:

```sh
 terraform apply -var admin_public_key="$(cat ~/.ssh/id_rsa_compound_chain.pub)" tf
```

### Setting up Compound Chain application

Lorem ipsum
