variable "key_spec" {
  description = "which curve u want?"
}

variable "alias" {
  description = "alias of the key"
}

# create role that reads
# give that role to ec2
data "aws_caller_identity" "current" {}

data "aws_iam_policy_document" "assume_role_policy" {
  statement {
    effect = "Allow"
    principals {
      type = "Service"
      identifiers = ["ec2.amazonaws.com"]
    }
    actions = ["sts:AssumeRole"]
  }
}

resource "aws_iam_role" "key_user_role" {
  name = "iam-role-for-signing-access"
  assume_role_policy = data.aws_iam_policy_document.assume_role_policy.json
}

resource "aws_iam_instance_profile" "authority" {
  name = "authority-role-signer"
  role = aws_iam_role.key_user_role.name
}

data "aws_iam_policy_document" "key_policy" {
  statement {
    sid    = "Enable IAM User Permissions"
    effect = "Allow"
    principals {
      type        = "AWS"
      identifiers = ["arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"]
    }
    actions   = ["kms:*"]
    resources = ["*"]
  }

  statement {
    sid    = "Allow Authority node to sign notices"
    effect = "Allow"
    principals {
      type        = "AWS"
      identifiers = [aws_iam_role.key_user_role.arn]
    }
    actions   = [
        "kms:DescribeKey",
        "kms:GetPublicKey",
        "kms:Sign",
        "kms:Verify"]
    resources = ["*"]
  }
}

resource "aws_kms_key" "kms_key" {
  description             = "KMS key for notice signing"
  enable_key_rotation     = false
  customer_master_key_spec = var.key_spec
  key_usage               = "SIGN_VERIFY"
  policy                  = data.aws_iam_policy_document.key_policy.json
}

resource "aws_kms_alias" "a" {
  name          = var.alias
  target_key_id = aws_kms_key.kms_key.key_id
}

output "instance_profile_for_access" {
  value = aws_iam_instance_profile.authority
}