#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Error {
    InvalidRecoveryId,
    RecoveryError,
}

const SIGNATURE_SIZE: usize = secp256k1::util::SIGNATURE_SIZE;
const FULL_SIGNATURE_SIZE: usize = secp256k1::util::SIGNATURE_SIZE + 1;
const RECOVERY_BIT: usize = secp256k1::util::SIGNATURE_SIZE;
const MESSAGE_SIZE: usize = secp256k1::util::MESSAGE_SIZE;
const RAW_PUBLIC_KEY_SIZE: usize = secp256k1::util::RAW_PUBLIC_KEY_SIZE;

pub fn get_chain(recovery_id: u8) -> Result<(u8, Option<u8>), Error> {
    match recovery_id {
        0..=1 => Ok((recovery_id, None)),
        27..=28 => Ok((recovery_id - 27, None)),
        35..=255 => Ok((1 - recovery_id % 2, Some((recovery_id - 35) / 2))),
        2..=26 | 29..=34 => Err(Error::InvalidRecoveryId),
    }
}

pub fn secp256k1_split_recovery_id(
    message: &[u8; FULL_SIGNATURE_SIZE],
) -> Result<([u8; SIGNATURE_SIZE], u8, Option<u8>), Error> {
    // TODO: Can we do this without a copy?
    let mut signature: [u8; SIGNATURE_SIZE] = [0u8; SIGNATURE_SIZE];
    signature.copy_from_slice(&message[0..SIGNATURE_SIZE]);
    let recovery_id: u8 = message[RECOVERY_BIT];

    let (recovery_id_bit, maybe_chain_id) = get_chain(recovery_id)?;

    Ok((signature, recovery_id_bit, maybe_chain_id))
}

pub fn secp256k1_recover(
    message: &[u8; MESSAGE_SIZE],
    signature: &[u8; SIGNATURE_SIZE],
    recovery_id: u8,
) -> Result<[u8; RAW_PUBLIC_KEY_SIZE], Error> {
    // Recover signature part
    let parsed_signature = secp256k1::Signature::parse(&signature);
    // Recover RecoveryId part
    let parsed_recovery_id =
        secp256k1::RecoveryId::parse(recovery_id).map_err(|_| Error::InvalidRecoveryId)?;

    // Recover validator's public key from signature
    let parsed_message = secp256k1::Message::parse(message);
    let signer = secp256k1::recover(&parsed_message, &parsed_signature, &parsed_recovery_id)
        .map_err(|_| Error::RecoveryError)?;

    // TODO: This is annoying, but we want the raw public key, which isn't available from the exposed API
    let mut pk: [u8; RAW_PUBLIC_KEY_SIZE] = [0u8; RAW_PUBLIC_KEY_SIZE];
    pk.copy_from_slice(&signer.serialize()[1..]);
    Ok(pk)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SIGNATURE: [u8; 64] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
        49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
    ];
    const TEST_SIGNATURE_WITH_RECOVERY_BIT: [u8; 65] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
        49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 40,
    ];

    #[test]
    fn test_get_chain() {
        assert_eq!(get_chain(0), Ok((0u8, None)));

        assert_eq!(get_chain(1), Ok((1u8, None)));

        assert_eq!(get_chain(27), Ok((0u8, None)));

        assert_eq!(get_chain(28), Ok((1u8, None)));

        assert_eq!(get_chain(35), Ok((0u8, Some(0))));

        assert_eq!(get_chain(36), Ok((1u8, Some(0))));

        assert_eq!(get_chain(39), Ok((0u8, Some(2))));

        assert_eq!(get_chain(40), Ok((1u8, Some(2))));

        assert_eq!(get_chain(10), Err(Error::InvalidRecoveryId));
    }

    #[test]
    fn test_secp256k1_split_recovery_id() {
        assert_eq!(
            secp256k1_split_recovery_id(&TEST_SIGNATURE_WITH_RECOVERY_BIT),
            Ok((TEST_SIGNATURE, 1, Some(2)))
        );
    }

    // TODO: Write more tests here
}
