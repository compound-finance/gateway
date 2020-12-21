use crate::{account::*, mock::*, Error};
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};

fn andrew() -> AccountIdent {
    AccountIdent {
        chain: ChainIdent::Eth,
        address: [123; 20].to_vec(),
    }
}

#[test]
fn it_fails_magic_extract_signed() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::magic_extract(Origin::signed(Default::default()), andrew(), 42),
            DispatchError::BadOrigin
        );
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), None);
    });
}

#[test]
fn it_magically_extracts() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_ok!(CashModule::magic_extract(Origin::none(), andrew(), 42));
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), Some(42));

        // Dispatch a second extrinsic.
        assert_ok!(CashModule::magic_extract(Origin::none(), andrew(), 42));
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), Some(84));
    });
}

#[test]
fn it_works_for_default_value() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_ok!(CashModule::process_eth_event(
            Origin::signed(Default::default()),
            vec![]
        ));
        // Read pallet storage and assert an expected result.
        // XXX assert_eq!(CashModule::something(), Some(42));
    });
}

#[test]
fn correct_error_for_none_value() {
    new_test_ext().execute_with(|| {
        // Ensure the expected error is thrown when no value is present.
        assert_noop!(
            CashModule::cause_error(Origin::signed(Default::default())),
            Error::<Test>::NoneValue
        );
    });
}
