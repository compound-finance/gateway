#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module,};

use frame_system::{
    Call,
    Module,
    offchain::SignedPayload,
};

use sp_runtime::transaction_validity::{
        InvalidTransaction,
        TransactionSource,
        TransactionValidity
    };

mod notices;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

extern crate ethereum_client;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_event!(
    pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
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
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        #[weight = 0]
        pub fn do_something(origin) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            
            Ok(())
        }

        // fn offchain_worker() {
        //     // process_requests();
        //     // process_notice_queue();
        // }

        pub fn process_requests() {
            let config = runtime_interfaces::config_interface::get();
            let eth_rpc_url = String::from_utf8(config.get_eth_rpc_url()).unwrap();
            // debug::native::info!("CONFIG: {:?}", eth_rpc_url);

            // TODO create parameter vector from storage variables
            let lock_events: Result<Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>, http::Error> = ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec!["{\"address\": \"0x3f861853B41e19D5BBe03363Bb2f50D191a723A2\", \"fromBlock\": \"0x146A47D\", \"toBlock\" : \"latest\", \"topics\":[\"0xddd0ae9ae645d3e7702ed6a55b29d04590c55af248d51c92c674638f3fb9d575\"]}"]);
            debug::native::info!("Lock Events: {:?}", lock_events);
        }

        pub fn process_notice_queue() {
            let pending_notices = <Vec<Notice>>

            // notice queue stub
            pending_notices = [];

            for notice in pending_notices.iter() {
                // find parent
                // id = notice.gen_id(parent)
                let message = encode(&notice);
                // let sig_result = Signer.send_unsigned_transaction(
                //     |account| NoticePayload {
                //         // id: move id,
                //         msg: message.clone(),
                //         sig: sign(&message),
                //         public: account.public.clone(),
                //     },
                //     Call::emit_notice);

                // match sig_result {
                //     Some((_, res)) => res.map_err(|_| {
                //         debug::error!("Failed in offchain_unsigned_tx_signed_payload");
                //         <Error<T>>::OffchainUnsignedTxSignedPayloadError
                //     }),
                //     None => {
                //         // The case of `None`: no account is available for sending
                //         debug::error!("No local account available");
                //         Err(<Error<T>>::NoLocalAcctForSigning)
                //     }
                // }
            }
        }

        #[weight = 0]
        pub fn emit_notice(origin, notice: NoticePayload, signature: T::Signature) -> DispatchResult {
            debug::native::info!("Extraction Notice Payload: {:?}", extraction_notice_payload);
            // TODO: Move to using unsigned and getting author from signature
            // TODO I don't know what this comment means ^
            ensure_none(origin)?;

            Self::deposit_event(RawEvent::Notice(notice.msg, notice.sig, notice.public, notice.id));

            Ok(())
        }
    }
}

#[allow(deprecated)] // ValidateUnsigned
impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
  type Call = Call<T>;

  /// Validate unsigned call to this module.
  ///
  /// By default unsigned transactions are disallowed, but implementing the validator
  /// here we make sure that some particular calls (the ones produced by offchain worker)
  /// are being whitelisted and marked as valid.
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
