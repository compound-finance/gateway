use crate::std::*;
use crate::CryptoError;
use rusoto_core::Region;
use rusoto_kms::KmsClient;

struct KmsKeyring {
    client: KmsClient,
}

impl Keyring for KmsKeyring {
    fn sign(
        self: &Self,
        messages: Vec<Vec<u8>>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<Vec<u8>, CryptoError>>, CryptoError> {
        unimplemented!()
    }

    fn get_public_key(self: &Self, key_id: &KeyId) -> Result<Vec<u8>, CryptoError> {
        unimplemented!()
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

        KmsKeyring { client }
    }
}
