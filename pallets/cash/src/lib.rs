#![cfg_attr(not(feature = "std"), no_std)]

use codec::alloc::string::String;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch
};
use frame_system::{
    ensure_none,
    offchain::SignedPayload
};
use sp_runtime::offchain::http;
use sp_runtime::{
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity
    },
};
use sp_std::vec::Vec;

extern crate ethereum_client;

mod notices;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate alloc;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
    trait Store for Module<T: Config> as CashPalletModule {
    }
}

decl_event!(
    // why does this depend on the un used account id binding? ( wont compile if deleted)
    pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
        Hi(AccountId),
        Notice(notices::Message, notices::Signature, notices::Address),
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
        pub fn emit_notice(origin, notice: notices::NoticePayload, ) -> dispatch::DispatchResult {
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
        let pending_notices = <Vec<Notice>>;

        // notice queue stub
        pending_notices = [];

        for notice in pending_notices.iter() {
            // find parent
            // id = notice.gen_id(parent)
            let message = encode(&notice);
            Signer.send_unsigned_transaction(
                move |account| NoticePayload {
                    // id: move id,
                    msg: message.clone(),
                    sig: sign(&message),
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
              // TODO: other validation? 
              // Self::validate_extraction_notice_transaction_parameters(
              //     &payload.msg,
              //     &payload.sig,
              //     &payload.who,
              // )
          }
          _ => InvalidTransaction::Call.into(),
      }
  }
}

