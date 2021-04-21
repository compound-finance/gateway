use crate::{
    reason::{MathError, Reason},
    types::{
        AssetAmount, AssetBalance, AssetQuantity, CashPrincipal, CashPrincipalAmount, Quantity,
    },
};
use our_std::{cmp::min, convert::TryFrom};

// XXX use Balances instead of raw balances everywhere and put all fns on types?

/// Adds an asset quantity to a given unsigned amount
pub fn add_amount_to_raw(a: AssetAmount, b: AssetQuantity) -> Result<AssetAmount, MathError> {
    Ok(a.checked_add(b.value).ok_or(MathError::Overflow)?)
}

/// Subtracts an asset quantity from a given unsigned amount, returning an unsigned amount
/// TODO: Remove underflow param here and just return a math error
pub fn sub_amount_from_raw(
    a: AssetAmount,
    b: AssetQuantity,
    underflow: Reason,
) -> Result<AssetAmount, Reason> {
    Ok(a.checked_sub(b.value).ok_or(underflow)?)
}

/// Adds an asset quantity to a given signed balance
pub fn add_amount_to_balance(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<AssetBalance, MathError> {
    let signed = AssetBalance::try_from(amount.value).or(Err(MathError::Overflow))?;
    Ok(balance.checked_add(signed).ok_or(MathError::Overflow)?)
}

/// Subtracts an asset quantity to a given signed balance, returning a signed balance
pub fn sub_amount_from_balance(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<AssetBalance, MathError> {
    let signed = AssetBalance::try_from(amount.value).or(Err(MathError::Overflow))?;
    Ok(balance.checked_sub(signed).ok_or(MathError::Underflow)?)
}

/// Sums two cash unsigned principals
pub fn add_principal_amounts(
    a: CashPrincipalAmount,
    b: CashPrincipalAmount,
) -> Result<CashPrincipalAmount, MathError> {
    Ok(a.add(b)?)
}

/// Subtracts two cash unsigned principals, returning an unsigned result or given error
/// TODO: Simply return math error instead?
pub fn sub_principal_amounts(
    a: CashPrincipalAmount,
    b: CashPrincipalAmount,
    underflow: Reason,
) -> Result<CashPrincipalAmount, Reason> {
    Ok(a.sub(b).map_err(|_| underflow)?)
}

/// Returns value if it is above zero, otherwise 0.
pub fn pos_balance(balance: AssetBalance) -> AssetAmount {
    if balance > 0 {
        balance as AssetAmount // This is safe since we've already checked value is > 0
    } else {
        0
    }
}

/// Returns abs of value if it is less than zero, otherwise 0.
pub fn neg_balance(balance: AssetBalance) -> Result<AssetAmount, MathError> {
    if balance < 0 {
        Ok(balance.checked_abs().ok_or(MathError::Overflow)? as AssetAmount)
    } else {
        Ok(0)
    }
}

/// Given a signed balance and a infusion of that asset, determines how much will be
/// used to repay debt versus added to supply. E.g. do I need to repay some debt and the
/// rest is supply? Or am I still in debt? Or is it all supply? This judgement turns on
/// whether the amount causes the balance to cross the zero threshold.
pub fn repay_and_supply_amount(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<(AssetQuantity, AssetQuantity), Reason> {
    let Quantity {
        value: raw_amount,
        units,
    } = amount;
    let repay_amount = min(neg_balance(balance).map_err(Reason::MathError)?, raw_amount);
    let supply_amount = raw_amount
        .checked_sub(repay_amount)
        .expect("repay_amount ≤ raw_amount");
    Ok((
        Quantity::new(repay_amount, units),
        Quantity::new(supply_amount, units),
    ))
}

/// Given a signed balance and a reduction of that asset, determines how much will be
/// pulled from supply versus added to debt. This judgement turns on
/// whether the amount causes the balance to cross the zero threshold.
pub fn withdraw_and_borrow_amount(
    balance: AssetBalance,
    amount: AssetQuantity,
) -> Result<(AssetQuantity, AssetQuantity), Reason> {
    let Quantity {
        value: raw_amount,
        units,
    } = amount;
    let withdraw_amount = min(pos_balance(balance), raw_amount);
    let borrow_amount = raw_amount
        .checked_sub(withdraw_amount)
        .expect("withdraw_amount ≤ raw_amount");
    Ok((
        Quantity::new(withdraw_amount, units),
        Quantity::new(borrow_amount, units),
    ))
}

/// Given a signed balance of cash and a infusion of cash, determines how much will be
/// used to repay debt versus added to supply. This judgement turns on
/// whether the amount causes the balance to cross the zero threshold.
pub fn repay_and_supply_principal(
    balance: CashPrincipal,
    principal: CashPrincipalAmount,
) -> Result<(CashPrincipalAmount, CashPrincipalAmount), Reason> {
    let repay_principal = min(
        neg_balance(balance.0).map_err(Reason::MathError)?,
        principal.0,
    );
    let supply_principal = principal
        .0
        .checked_sub(repay_principal)
        .expect("repay_principal ≤ principal");
    Ok((
        CashPrincipalAmount(repay_principal),
        CashPrincipalAmount(supply_principal),
    ))
}

/// Given a signed balance of cash and a reduction of cash, determines how much will be
/// pulled from supply versus added to debt. This judgement turns on
/// whether the amount causes the balance to cross the zero threshold.
pub fn withdraw_and_borrow_principal(
    balance: CashPrincipal,
    principal: CashPrincipalAmount,
) -> Result<(CashPrincipalAmount, CashPrincipalAmount), Reason> {
    let withdraw_principal = min(pos_balance(balance.0), principal.0);
    let borrow_principal = principal
        .0
        .checked_sub(withdraw_principal)
        .expect("withdraw_principal ≤ principal");
    Ok((
        CashPrincipalAmount(withdraw_principal),
        CashPrincipalAmount(borrow_principal),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_add_amount_to_raw_ok() {
        assert_eq!(
            add_amount_to_raw(1000000, Quantity::from_nominal("5", USD)),
            Ok(6000000)
        );
    }

    #[test]
    fn test_add_amount_to_raw_overflow() {
        assert_eq!(
            add_amount_to_raw(u128::MAX, Quantity::from_nominal("5", USD)),
            Err(MathError::Overflow)
        );
    }

    #[test]
    fn test_sub_amount_from_raw_ok() {
        assert_eq!(
            sub_amount_from_raw(6000000, Quantity::from_nominal("5", USD), Reason::None),
            Ok(1000000)
        );
    }

    #[test]
    fn test_sub_amount_from_raw_underflow() {
        assert_eq!(
            sub_amount_from_raw(4000000, Quantity::from_nominal("5", USD), Reason::None),
            Err(Reason::None)
        );
    }

    #[test]
    fn test_add_amount_to_balance_ok() {
        assert_eq!(
            add_amount_to_balance(-1000000, Quantity::from_nominal("5", USD)),
            Ok(4000000)
        );
    }

    #[test]
    fn test_add_amount_to_balance_overflow() {
        assert_eq!(
            add_amount_to_balance(i128::MAX, Quantity::from_nominal("5", USD)),
            Err(MathError::Overflow)
        );
    }

    #[test]
    fn test_sub_amount_from_balance_ok() {
        assert_eq!(
            sub_amount_from_balance(5000000, Quantity::from_nominal("1", USD)),
            Ok(4000000)
        );
    }

    #[test]
    fn test_sub_amount_from_balance_ok_negative() {
        assert_eq!(
            sub_amount_from_balance(5000000, Quantity::from_nominal("6", USD)),
            Ok(-1000000)
        );
    }

    #[test]
    fn test_sub_amount_from_balance_underflow() {
        assert_eq!(
            sub_amount_from_balance(i128::MIN, Quantity::from_nominal("6", USD)),
            Err(MathError::Underflow)
        );
    }

    #[test]
    fn test_add_principal_amounts_ok() {
        assert_eq!(
            add_principal_amounts(
                CashPrincipalAmount::from_nominal("5"),
                CashPrincipalAmount::from_nominal("6")
            ),
            Ok(CashPrincipalAmount::from_nominal("11"),)
        );
    }

    #[test]
    fn test_add_principal_amounts_overflow() {
        assert_eq!(
            add_principal_amounts(
                CashPrincipalAmount::from_nominal("5"),
                CashPrincipalAmount(u128::MAX)
            ),
            Err(MathError::Overflow)
        );
    }

    #[test]
    fn test_sub_principal_amounts_ok() {
        assert_eq!(
            sub_principal_amounts(
                CashPrincipalAmount::from_nominal("6"),
                CashPrincipalAmount::from_nominal("5"),
                Reason::None
            ),
            Ok(CashPrincipalAmount::from_nominal("1"),)
        );
    }

    #[test]
    fn test_sub_principal_amounts_overflow() {
        assert_eq!(
            sub_principal_amounts(
                CashPrincipalAmount::from_nominal("5"),
                CashPrincipalAmount::from_nominal("6"),
                Reason::None
            ),
            Err(Reason::None)
        );
    }

    #[test]
    fn test_pos_balance_pos() {
        assert_eq!(pos_balance(5000), 5000);
    }

    #[test]
    fn test_pos_balance_zero() {
        assert_eq!(pos_balance(0), 0);
    }

    #[test]
    fn test_pos_balance_neg() {
        assert_eq!(pos_balance(-1000), 0);
    }

    #[test]
    fn test_neg_balance_pos() {
        assert_eq!(neg_balance(5000), Ok(0));
    }

    #[test]
    fn test_neg_balance_zero() {
        assert_eq!(neg_balance(0), Ok(0));
    }

    #[test]
    fn test_neg_balance_neg() {
        assert_eq!(neg_balance(-1000), Ok(1000));
    }

    #[test]
    fn test_neg_balance_very_neg() {
        assert_eq!(neg_balance(i128::MIN), Err(MathError::Overflow));
    }

    #[test]
    fn test_repay_and_supply_amount_all_supply() {
        assert_eq!(
            repay_and_supply_amount(1000000, Quantity::from_nominal("5", USD)),
            Ok((
                Quantity::from_nominal("0", USD),
                Quantity::from_nominal("5", USD)
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_amount_all_repay() {
        assert_eq!(
            repay_and_supply_amount(-6000000, Quantity::from_nominal("5", USD)),
            Ok((
                Quantity::from_nominal("5", USD),
                Quantity::from_nominal("0", USD)
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_amount_low_high() {
        assert_eq!(
            repay_and_supply_amount(
                i128::MIN + 1,
                Quantity {
                    value: u128::MAX,
                    units: USD
                }
            ),
            Ok((
                Quantity {
                    value: 170141183460469231731687303715884105727,
                    units: USD
                },
                Quantity {
                    value: 170141183460469231731687303715884105728,
                    units: USD
                }
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_amount_overflow() {
        assert_eq!(
            repay_and_supply_amount(i128::MIN, Quantity::from_nominal("5", USD)),
            Err(Reason::MathError(MathError::Overflow))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_amount_all_withdraw() {
        assert_eq!(
            withdraw_and_borrow_amount(6000000, Quantity::from_nominal("5", USD)),
            Ok((
                Quantity::from_nominal("5", USD),
                Quantity::from_nominal("0", USD)
            ))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_amount_all_borrow() {
        assert_eq!(
            withdraw_and_borrow_amount(-6000000, Quantity::from_nominal("5", USD)),
            Ok((
                Quantity::from_nominal("0", USD),
                Quantity::from_nominal("5", USD)
            ))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_amount_low_high() {
        assert_eq!(
            withdraw_and_borrow_amount(
                i128::MIN + 1,
                Quantity {
                    value: u128::MAX,
                    units: USD
                }
            ),
            Ok((
                Quantity {
                    value: 0,
                    units: USD
                },
                Quantity {
                    value: 340282366920938463463374607431768211455,
                    units: USD
                }
            ))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_amount_overflow() {
        assert_eq!(
            withdraw_and_borrow_amount(
                i128::MAX,
                Quantity {
                    value: u128::MAX,
                    units: USD
                }
            ),
            Ok((
                Quantity {
                    value: 170141183460469231731687303715884105727,
                    units: USD
                },
                Quantity {
                    value: 170141183460469231731687303715884105728,
                    units: USD
                }
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_principal_all_supply() {
        assert_eq!(
            repay_and_supply_principal(
                CashPrincipal::from_nominal("5"),
                CashPrincipalAmount::from_nominal("1")
            ),
            Ok((
                CashPrincipalAmount::from_nominal("0"),
                CashPrincipalAmount::from_nominal("1")
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_principal_all_repay() {
        assert_eq!(
            repay_and_supply_principal(
                CashPrincipal::from_nominal("-5"),
                CashPrincipalAmount::from_nominal("1")
            ),
            Ok((
                CashPrincipalAmount::from_nominal("1"),
                CashPrincipalAmount::from_nominal("0")
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_principal_low_high() {
        assert_eq!(
            repay_and_supply_principal(
                CashPrincipal(i128::MIN + 1),
                CashPrincipalAmount(u128::MAX)
            ),
            Ok((
                CashPrincipalAmount(170141183460469231731687303715884105727),
                CashPrincipalAmount(170141183460469231731687303715884105728)
            ))
        );
    }

    #[test]
    fn test_repay_and_supply_principal_overflow() {
        assert_eq!(
            repay_and_supply_principal(
                CashPrincipal(i128::MIN),
                CashPrincipalAmount::from_nominal("1")
            ),
            Err(Reason::MathError(MathError::Overflow))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_principal_all_withdraw() {
        assert_eq!(
            withdraw_and_borrow_principal(
                CashPrincipal::from_nominal("5"),
                CashPrincipalAmount::from_nominal("1")
            ),
            Ok((
                CashPrincipalAmount::from_nominal("1"),
                CashPrincipalAmount::from_nominal("0")
            ))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_principal_all_borrow() {
        assert_eq!(
            withdraw_and_borrow_principal(
                CashPrincipal::from_nominal("-5"),
                CashPrincipalAmount::from_nominal("1")
            ),
            Ok((
                CashPrincipalAmount::from_nominal("0"),
                CashPrincipalAmount::from_nominal("1")
            ))
        );
    }

    #[test]
    fn test_withdraw_and_borrow_principal_low_high() {
        assert_eq!(
            withdraw_and_borrow_principal(
                CashPrincipal(i128::MIN + 1),
                CashPrincipalAmount(u128::MAX)
            ),
            Ok((
                CashPrincipalAmount(0),
                CashPrincipalAmount(340282366920938463463374607431768211455)
            ))
        );
    }
}
