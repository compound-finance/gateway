use crate::std::*;
use crate::CryptoError;
use rusoto_core::Region;
use rusoto_kms::{KmsClient, Kms, GetPublicKeyRequest};
use tokio::runtime::Runtime;

struct KmsKeyring {
    client: KmsClient,
    runtime: tokio::runtime::Runtime,
}

impl Keyring for KmsKeyring {
    fn sign(
        self: &Self,
        messages: Vec<Vec<u8>>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<Vec<u8>, CryptoError>>, CryptoError> {
        unimplemented!()
    }

    fn get_public_key(self: &mut Self, key_id: &KeyId) -> Result<Vec<u8>, CryptoError> {
        self.runtime.block_on(self.get_public_key_async(key_id))
    }
}

impl KmsKeyring {
    /// Create a new KMS keyring. Standard methods of configuring AWS clients within the
    /// environment are observed. Use environment variables AWS_REGION or AWS_DEFAULT_REGION to set
    /// the region use AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY OR AWS_SESSION_TOKEN and
    /// AWS_CREDENTIAL_EXPIRATION to authenticate.
    fn new() -> KmsKeyring {
        let region = Region::default();
        let client = KmsClient::new(region);

        // todo: It is unclear to me under what circumstances tokio runtime creation fails, for now.. unwrap..?
        KmsKeyring {
            client,
            runtime: tokio::runtime::Runtime::new().unwrap(),
        }
    }

    async fn get_public_key_async(self: &mut Self, key_id: &KeyId) -> Result<Vec<u8>, CryptoError> {
        let request : GetPublicKeyRequest = GetPublicKeyRequest{key_id: key_id.clone().into(), ..Default::default()};
        let result = self.client.get_public_key(request).await.map_err(|_|CryptoError::KeyNotFound)?;

    }
}
