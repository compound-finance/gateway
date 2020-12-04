#![cfg_attr(not(feature = "std"), no_std)]
use codec::alloc::string::String;
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch
};
use frame_system::{
    ensure_none,
    offchain::{ AppCrypto, CreateSignedTransaction, SendTransactionTypes, SendUnsignedTransaction, SignedPayload, Signer, SigningTypes }
};
use sp_core::crypto::{KeyTypeId};
use sp_runtime::offchain::http;
use sp_runtime::{
    transaction_validity::{
        ValidTransaction,
        InvalidTransaction,
        TransactionSource,
        TransactionValidity
    },
};
use sp_std::vec::Vec;

mod notices;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate alloc;
extern crate ethereum_client;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"gran");
pub const ETH_KEY_TYPE: KeyTypeId = KeyTypeId(*b"eth!");

pub mod crypto {
    use crate::KEY_TYPE;
    use sp_core::ed25519::Signature as Ed25519Signature;

    use sp_runtime::app_crypto::{app_crypto, ed25519};
    use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

    app_crypto!(ed25519, KEY_TYPE);

    pub struct TestAuthId;
    // implemented for ocw-runtime
    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::ed25519::Signature;
        type GenericPublic = sp_core::ed25519::Public;
    }

    impl frame_system::offchain::AppCrypto<<Ed25519Signature as Verify>::Signer, Ed25519Signature>
        for TestAuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::ed25519::Signature;
        type GenericPublic = sp_core::ed25519::Public;
    }
}

pub mod ecdsa_crypto {
    use crate::ETH_KEY_TYPE;
    use sp_core::ecdsa::Signature as EcdsaSignature;

    use sp_runtime::app_crypto::{app_crypto, ecdsa};
    use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

    app_crypto!(ecdsa, ETH_KEY_TYPE);

    pub struct TestEthAuthId;

    // implemented for ocw-runtime
    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestEthAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::ecdsa::Signature;
        type GenericPublic = sp_core::ecdsa::Public;
    }

    impl frame_system::offchain::AppCrypto<<EcdsaSignature as Verify>::Signer, EcdsaSignature>
        for TestEthAuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::ecdsa::Signature;
        type GenericPublic = sp_core::ecdsa::Public;
    }
}

pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> + SendTransactionTypes<<Self as Config>::Call> {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

     /// The identifier type for an offchain worker.
     type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

     /// The overarching dispatch call type.
     type Call: From<Call<Self>>;

    //  type SubmitUnsignedTransaction: frame_system::offchain::SendUnsignedTransaction<Self, <Self as Config>::Call>;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
    }
}

decl_event!(
    // why does this depend on the un used account id binding? ( wont compile if deleted)
    pub enum Event<T> where Public = <T as SigningTypes>::Public {
        Notice(notices::Message, notices::Signature, Public),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        NoneValue,
        StorageOverflow,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        fn offchain_worker(block_number: T::BlockNumber) {
            Self::process_requests();
            Self::process_notices();
        }

        #[weight = 0]
        pub fn emit_notice(origin, notice: notices::NoticePayload<T::Public>, _signature: T::Signature) -> dispatch::DispatchResult {
            // TODO: Move to using unsigned and getting author from signature
            // TODO I don't know what this comment means ^
            ensure_none(origin)?;

            Self::deposit_event(RawEvent::Notice(notice.msg, notice.sig, notice.public));

            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    fn process_requests() {
        debug::native::info!("Hello World from offchain workers!");
        let config = runtime_interfaces::config_interface::get();
        let eth_rpc_url = String::from_utf8(config.get_eth_rpc_url()).unwrap();
        // debug::native::info!("CONFIG: {:?}", eth_rpc_url);

        // TODO create parameter vector from storage variables
        let lock_events: Result<Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>, http::Error> = ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec!["{\"address\": \"0x3f861853B41e19D5BBe03363Bb2f50D191a723A2\", \"fromBlock\": \"0x146A47D\", \"toBlock\" : \"latest\", \"topics\":[\"0xddd0ae9ae645d3e7702ed6a55b29d04590c55af248d51c92c674638f3fb9d575\"]}"]);
        debug::native::info!("Lock Events: {:?}", lock_events);
    }

    pub fn process_notices() {
        // notice queue stub
        let pending_notices: Vec<&dyn notices::Notice> = [].to_vec();

        let signer = Signer::<T, T::AuthorityId>::any_account();
        for notice in pending_notices.iter() {
            // find parent
            // id = notice.gen_id(parent)
            let message = notice.encode();
            signer.send_unsigned_transaction(
                |account| notices::NoticePayload {
                    // id: move id,
                    msg: message.clone(),
                    sig: notices::sign(&message),
                    public: account.public.clone(),
                },
                Call::emit_notice);
        }
    }
}


#[allow(deprecated)] // ValidateUnsigned
impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
  type Call = Call<T>;

  fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
      match call {
          Call::emit_notice(ref payload, ref signature) => {
              let signature_valid =
                  SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
              if !signature_valid {
                  return InvalidTransaction::BadProof.into();
              }
              ValidTransaction::with_tag_prefix("Cash")
              // The transaction is only valid for next 10 blocks. After that it's
              // going to be revalidated by the pool.
                  // XXX: ? who knows how long ? - CB
                  .longevity(10)
                  // XXX: don't propogate, just expect to get it yourself next time? -CB
                  .propagate(false)
                  .build()
          }
          _ => InvalidTransaction::Call.into(),
      }
  }
}

