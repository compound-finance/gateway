use crate::std::*;
use crate::{
    eth_keccak_for_signature, tagged_public_key_slice_to_raw, CryptoError, HashedMessageBytes,
    PublicKeyBytes, SignatureBytes, ETH_ADD_TO_V,
};
use der_parser::parse_der;
use rusoto_core::{Region, RusotoError};
use rusoto_kms::{GetPublicKeyRequest, Kms, KmsClient, SignError, SignRequest, SignResponse};
use secp256k1::util::{FULL_PUBLIC_KEY_SIZE, TAG_PUBKEY_FULL};
use secp256k1::{PublicKey, PublicKeyFormat, RecoveryId, Signature};

/// Store your keys in AWS Key Management Service (KMS) for increased security. KMS is implemented
/// using Hardware Signing Modules (HSMs) for the highest level of security. It is relatively inexpensive
/// and available to all AWS accounts in all regions.
///
/// Some very useful links
/// https://luhenning.medium.com/the-dark-side-of-the-elliptic-curve-signing-ethereum-transactions-with-aws-kms-in-javascript-83610d9a6f81
/// https://github.com/lucashenning/aws-kms-ethereum-signing
pub struct KmsKeyring {
    client: KmsClient,
}
use tokio::runtime::Runtime;
impl Keyring for KmsKeyring {
    /// Sign messages using the AWS KMS HSM system. The key id can be an ARN, an AWS key id (uuid)
    /// or a key alias.
    fn sign(
        self: &Self,
        messages: Vec<&[u8]>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<SignatureBytes, CryptoError>>, CryptoError> {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(self.sign_async(messages, key_id))
    }

    fn sign_one(self: &Self, messages: &[u8], key_id: &KeyId) -> Result<[u8; 65], CryptoError> {
        // we will use the batch interface
        self.sign(vec![messages], key_id)?
            .drain(..)
            .next()
            .ok_or(CryptoError::Unknown)?
    }

    /// Get the public key corresponding to the provided key ID.
    fn get_public_key(self: &Self, key_id: &KeyId) -> Result<PublicKeyBytes, CryptoError> {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(self.get_public_key_async(key_id))
    }
}

const KMS_SIGNING_ALGORITHM_ECDSA_SHA_256: &str = "ECDSA_SHA_256";
const KMS_MESSAGE_TYPE_DIGEST: &str = "DIGEST";

impl KmsKeyring {
    /// Create a new KMS keyring. Standard methods of configuring AWS clients within the
    /// environment are observed. Use environment variables AWS_REGION or AWS_DEFAULT_REGION to set
    /// the region use AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY OR AWS_SESSION_TOKEN and
    /// AWS_CREDENTIAL_EXPIRATION to authenticate.
    pub fn new() -> KmsKeyring {
        let region = Region::default();
        let client = KmsClient::new(region);

        // todo: It is unclear to me under what circumstances tokio runtime creation fails, for now.. unwrap..?
        KmsKeyring { client: client }
    }

    /// Get the public key corresponding to the key_id from KMS.
    async fn get_public_key_async(
        self: &Self,
        key_id: &KeyId,
    ) -> Result<PublicKeyBytes, CryptoError> {
        let request: GetPublicKeyRequest = GetPublicKeyRequest {
            key_id: key_id.clone().into(),
            ..Default::default()
        };
        let result = self
            .client
            .get_public_key(request)
            .await
            .map_err(|_| CryptoError::KeyNotFound)?;
        let public_key = result.public_key.ok_or(CryptoError::KeyNotFound)?;
        let (_, decoded) = parse_der(&public_key).map_err(|_| CryptoError::ParseError)?;
        let sequence = decoded.as_sequence().map_err(|_| CryptoError::ParseError)?;
        if sequence.len() != 2 {
            return Err(CryptoError::ParseError);
        }
        let actual_public_key = sequence[1]
            .content
            .as_slice()
            .map_err(|_| CryptoError::ParseError)?;
        if actual_public_key.len() != FULL_PUBLIC_KEY_SIZE {
            return Err(CryptoError::ParseError);
        }
        // strip first byte due to https://tools.ietf.org/html/rfc5480#section-2.2 indicating "uncompressed"
        // this could be considered "RAW" format
        if actual_public_key[0] != TAG_PUBKEY_FULL {
            return Err(CryptoError::ParseError);
        }

        // let address = public_key_bytes_to_eth_address(actual_public_key);
        Ok(tagged_public_key_slice_to_raw(actual_public_key)?)
    }

    /// Sign the messages asynchronously. This submits multiple requests to the HSM in parallel
    /// one for each message.
    async fn sign_async(
        self: &Self,
        messages: Vec<&[u8]>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<SignatureBytes, CryptoError>>, CryptoError> {
        // launch all of the tasks
        let public_key_promise = self.get_public_key_async(&key_id);
        let mut requests = Vec::new();
        for message in messages {
            let hashed = eth_keccak_for_signature(&message, false);
            let request = SignRequest {
                key_id: key_id.into(),
                message: bytes::Bytes::copy_from_slice(&hashed),
                message_type: Some(KMS_MESSAGE_TYPE_DIGEST.into()),
                signing_algorithm: KMS_SIGNING_ALGORITHM_ECDSA_SHA_256.into(),
                ..Default::default()
            };
            let fut = self.client.sign(request);
            requests.push((fut, hashed));
        }

        let public_key = public_key_promise
            .await
            .map_err(|_| CryptoError::KeyNotFound)?;

        // collect results
        let mut all_results = Vec::new();
        for (fut, hashed) in requests {
            // note, we can await serially here because all tasks have already been launched
            let resolved = fut.await;
            let signature = Self::result_to_signature(resolved, &public_key, hashed);
            all_results.push(signature);
        }

        Ok(all_results)
    }

    /// Convert the output from AWS KMS Signature function to our conventions
    /// See documentation here on their output
    /// https://docs.aws.amazon.com/kms/latest/APIReference/API_Sign.html#API_Sign_ResponseSyntax
    fn result_to_signature(
        resolved: Result<SignResponse, RusotoError<SignError>>,
        public_key: &[u8],
        digested: HashedMessageBytes,
    ) -> Result<SignatureBytes, CryptoError> {
        // Parse the signature into something usable, lots of ways this can fail.
        let der_encoded_signature = resolved
            .map_err(|_| CryptoError::HSMError)?
            .signature
            .ok_or(CryptoError::HSMError)?
            .to_vec();

        let mut sig =
            Signature::parse_der(&der_encoded_signature).map_err(|_| CryptoError::ParseError)?;
        // Because of EIP-2 not all elliptic curve signatures are accepted
        // the value of s needs to be SMALLER than half of the curve
        // i.e. we need to flip s if it's greater than half of the curve
        if sig.s.is_high() {
            sig.s = -sig.s;
        }

        // find the recovery id by guessing
        let public_key = PublicKey::parse_slice(public_key, Some(PublicKeyFormat::Raw))
            .map_err(|_| CryptoError::ParseError)?;
        let message =
            secp256k1::Message::parse_slice(&digested).map_err(|_| CryptoError::ParseError)?;
        let mut recovery_id = RecoveryId::parse(0).map_err(|_| CryptoError::ParseError)?;
        let recovered = secp256k1::recover(&message, &sig, &recovery_id)
            .map_err(|_| CryptoError::RecoverError)?;
        if recovered != public_key {
            recovery_id = RecoveryId::parse(1).map_err(|_| CryptoError::ParseError)?
        }

        // combine signature with recovery ID so we can recover the public key of the signer later
        Ok(combine_sig_and_recovery(
            sig.serialize(),
            recovery_id.serialize() + ETH_ADD_TO_V,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eth_recover;

    fn get_test_setup() -> (KmsKeyring, KeyId) {
        let keyring = KmsKeyring::new();
        let key_id = KeyId::from(
            "arn:aws:kms:us-east-1:376470027280:key/336459de-d7a4-41a9-a900-ca34a1559daa",
        );

        (keyring, key_id)
    }

    #[test]
    #[ignore]
    fn test_get_public_key() {
        let (keyring, key_id) = get_test_setup();
        let pk = keyring.get_public_key(&key_id).unwrap();
        assert!(pk.len() > 0);
    }

    #[test]
    #[ignore]
    fn test_sign() {
        let (keyring, key_id) = get_test_setup();
        let message: Vec<u8> = "hello".into();
        let messages: Vec<&[u8]> = vec![&message];
        let message_len = messages.len();

        let mut result = keyring.sign(messages, &key_id).unwrap();
        assert_eq!(result.len(), message_len);
        let sig = result.drain(..).next().unwrap().unwrap();
        assert_eq!(sig.len(), 65);

        // to verify, check that the address matches when you run recover
        let expected_address = eth_recover(&message, &sig, true).unwrap();
        let actual_public_key = keyring.get_public_key(&key_id).unwrap();
        let actual_address = crate::public_key_bytes_to_eth_address(&actual_public_key);
        assert_eq!(expected_address, actual_address, "address mismatch");
        //assert_eq!(sig, vec![0u8]);
    }
}
