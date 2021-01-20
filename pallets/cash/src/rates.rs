/// Interest rate related calculations and utilities are concentrated here
use crate::core::{GenericQty, Uint};
use codec::{Decode, Encode};
use our_std::Debuggable;

/// Error enum for interest rates
#[derive(Debuggable, PartialEq, Eq, Encode, Decode)]
pub enum RatesError {
    UtilizationNumeratorOverflow,
    UtilizationDivisorOverflow,
    UtilizationZeroSupplyError,
    UtilizationBorrowedIsMoreThanSupplied,
    ModelNotIncreasingZeroRateAboveKinkRate,
    ModelNotIncreasingKinkRateAboveFullRate,
    ModelKinkUtilizationOver100Percent,
    ModelKinkUtilizationNotPositive,
    ModelRateOutOfBounds,
    ModelNumeratorOverflow,
    ModelDivisionOverflow,
    ModelRateImpossiblyHigh,
    ModelProgrammerError,
}

/// Utilization rate for a given market.
pub type Utilization = Uint;

/// Interest rate
pub type APR = Uint;

const UTILIZATION_ONE: Uint = 10000;

const RATE_ONE: Uint = 10000;

const MAX_RATE: Uint = 3500; // 35%

/// Get the utilization ratio given the amount supplied and borrowed. These amounts should be in
/// "today" money AKA "balance" money.
pub fn get_utilization(
    supplied: GenericQty,
    borrowed: GenericQty,
) -> Result<Utilization, RatesError> {
    if supplied == 0 {
        return Err(RatesError::UtilizationZeroSupplyError);
    }
    if borrowed > supplied {
        return Err(RatesError::UtilizationBorrowedIsMoreThanSupplied);
    }
    let (numerator, overflowed) = borrowed.overflowing_mul(UTILIZATION_ONE);
    if overflowed {
        return Err(RatesError::UtilizationNumeratorOverflow);
    }
    let (result, overflowed) = numerator.overflowing_div(supplied);
    if overflowed {
        // note - unfortunately with u128 this branch is not testable
        return Err(RatesError::UtilizationDivisorOverflow);
    }

    Ok(result)
}

/// This represents an interest rate model type and parameters.
/// It also implements the pure functionality of the interest rate model
/// leaving out all issues of storage read and write.
///
/// In the future we may support serde serialization and deserialization of this struct
/// for the purpose of inclusion in the genesis configuration chain_spec file.
#[derive(Debuggable, Encode, Decode, PartialEq, Eq, Copy, Clone)]
pub enum InterestRateModel {
    Kink {
        zero_rate: APR,
        kink_rate: APR,
        kink_utilization: Utilization,
        full_rate: APR,
    },
}

/// This is _required_ for storage, we should never depend on this.
impl Default for InterestRateModel {
    fn default() -> Self {
        InterestRateModel::Kink {
            zero_rate: 0,
            kink_rate: 500,
            kink_utilization: 8000,
            full_rate: 2000,
        }
    }
}

impl InterestRateModel {
    pub fn new_kink(
        zero_rate: APR,
        kink_rate: APR,
        kink_utilization: Utilization,
        full_rate: APR,
    ) -> InterestRateModel {
        InterestRateModel::Kink {
            zero_rate,
            kink_rate,
            kink_utilization,
            full_rate,
        }
    }

    /// Check the model parameters for sanity
    ///
    /// Kink - we expect the kink model to be monotonically increasing with a kink somewhere between
    /// 0% and 100% utilization
    pub fn check_parameters(self: &Self) -> Result<(), RatesError> {
        match self {
            Self::Kink {
                zero_rate,
                kink_rate,
                kink_utilization,
                full_rate,
            } => {
                if *zero_rate > MAX_RATE || *kink_rate > MAX_RATE || *full_rate > MAX_RATE {
                    return Err(RatesError::ModelRateOutOfBounds);
                }

                if zero_rate >= kink_rate {
                    return Err(RatesError::ModelNotIncreasingZeroRateAboveKinkRate);
                }

                if kink_rate >= full_rate {
                    return Err(RatesError::ModelNotIncreasingKinkRateAboveFullRate);
                }

                if *kink_utilization >= UTILIZATION_ONE {
                    return Err(RatesError::ModelKinkUtilizationOver100Percent);
                }

                if *kink_utilization <= 0 {
                    return Err(RatesError::ModelKinkUtilizationNotPositive);
                }
            }
        };

        Ok(())
    }

    /// Get the borrow rate
    /// Current rate is not used at the moment
    pub fn get_borrow_rate(
        self: &Self,
        utilization: Utilization,
        current_rate: APR,
    ) -> Result<APR, RatesError> {
        match self {
            Self::Kink {
                zero_rate,
                kink_rate,
                kink_utilization,
                full_rate,
            } => {
                if utilization < *kink_utilization {
                    // left side of the kink
                    // unsafe -> return Ok(utilization * (kink_rate - zero_rate) / kink_utilization + zero_rate);

                    let (rate_difference, overflowed) = kink_rate.overflowing_sub(*zero_rate);
                    if overflowed {
                        return Err(RatesError::ModelNotIncreasingZeroRateAboveKinkRate);
                    }

                    let (numerator, overflowed) = utilization.overflowing_mul(rate_difference);
                    if overflowed {
                        return Err(RatesError::ModelNumeratorOverflow);
                    }

                    let (rise, overflowed) = numerator.overflowing_div(*kink_utilization);
                    if overflowed {
                        return Err(RatesError::ModelDivisionOverflow);
                    }

                    let (result, overflowed) = rise.overflowing_add(*zero_rate);
                    if overflowed {
                        return Err(RatesError::ModelRateImpossiblyHigh);
                    }

                    return Ok(result);
                } else {
                    // unsafe -> return Ok( (utilization - kink_utilization)*(full_rate - kink_rate) / ( 1 - kink_utilization) + kink_rate
                    let (utilization_difference, overflowed) =
                        utilization.overflowing_sub(*kink_utilization);
                    if overflowed {
                        return Err(RatesError::ModelProgrammerError);
                    }

                    let (slope_rise, overflowed) = full_rate.overflowing_sub(*kink_rate);
                    if overflowed {
                        return Err(RatesError::ModelNotIncreasingKinkRateAboveFullRate);
                    }

                    let (numerator, overflowed) =
                        utilization_difference.overflowing_mul(slope_rise);
                    if overflowed {
                        return Err(RatesError::ModelNumeratorOverflow);
                    }

                    let (slope_run, overflowed) =
                        UTILIZATION_ONE.overflowing_sub(*kink_utilization);
                    if overflowed {
                        return Err(RatesError::ModelKinkUtilizationOver100Percent);
                    }

                    let (rise, overflowed) = numerator.overflowing_div(slope_run);
                    if overflowed {
                        return Err(RatesError::ModelDivisionOverflow);
                    }

                    let (result, overflowed) = rise.overflowing_add(*kink_rate);
                    if overflowed {
                        return Err(RatesError::ModelRateImpossiblyHigh);
                    }

                    return Ok(result);
                }
            }
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct UtilizationTestCase {
        supplied: GenericQty,
        borrowed: GenericQty,
        expected: Result<Utilization, RatesError>,
        message: &'static str,
    }

    struct InterestRateModelCheckParametersTestCase {
        model: InterestRateModel,
        expected: Result<(), RatesError>,
        message: &'static str,
    }

    struct InterestRateModelGetBorrowRateTestCase {
        model: InterestRateModel,
        utilization: Utilization,
        expected: Result<APR, RatesError>,
        message: &'static str,
    }

    fn get_utilization_test_cases() -> Vec<UtilizationTestCase> {
        vec![
            UtilizationTestCase {
                supplied: 0,
                borrowed: 0,
                expected: Err(RatesError::UtilizationZeroSupplyError),
                message: "Zero supply implies an undefined utilization",
            },
            UtilizationTestCase {
                supplied: 1,
                borrowed: 2,
                expected: Err(RatesError::UtilizationBorrowedIsMoreThanSupplied),
                message: "Borrowed can not be more than supplied",
            },
            UtilizationTestCase {
                supplied: Uint::max_value(),
                borrowed: Uint::max_value(),
                expected: Err(RatesError::UtilizationNumeratorOverflow),
                message: "These numbers are vastly too large to compute the utilization",
            },
            UtilizationTestCase {
                supplied: Uint::max_value() / UTILIZATION_ONE + 1,
                borrowed: Uint::max_value() / UTILIZATION_ONE + 1,
                expected: Err(RatesError::UtilizationNumeratorOverflow),
                message: "These numbers are only just too large to compute the utilization",
            },
            UtilizationTestCase {
                supplied: Uint::max_value() / UTILIZATION_ONE,
                borrowed: Uint::max_value() / UTILIZATION_ONE,
                expected: Ok(UTILIZATION_ONE),
                message: "These are the largest numbers we can use to compute the utilization",
            },
            UtilizationTestCase {
                supplied: 100,
                borrowed: 100,
                expected: Ok(UTILIZATION_ONE),
                message: "This is a basic test of 100% utilization",
            },
            UtilizationTestCase {
                supplied: 100,
                borrowed: 0,
                expected: Ok(0),
                message: "A basic test of 0 utilization",
            },
            UtilizationTestCase {
                supplied: 100,
                borrowed: 50,
                expected: Ok(UTILIZATION_ONE / 2),
                message: "A basic test of middling utilization",
            },
        ]
    }

    fn test_get_utilizatio_case(case: UtilizationTestCase) {
        assert_eq!(
            case.expected,
            get_utilization(case.supplied, case.borrowed),
            "{}",
            case.message
        );
    }

    #[test]
    fn test_get_utilization() {
        get_utilization_test_cases()
            .drain(..)
            .for_each(test_get_utilizatio_case)
    }

    fn get_check_parameters_test_cases() -> Vec<InterestRateModelCheckParametersTestCase> {
        vec![
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: 2,
                    full_rate: 3,
                    kink_utilization: UTILIZATION_ONE / 2,
                },
                expected: Ok(()),
                message: "typical case should work well",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: 1,
                    full_rate: 3,
                    kink_utilization: UTILIZATION_ONE / 2,
                },
                expected: Err(RatesError::ModelNotIncreasingZeroRateAboveKinkRate),
                message: "rates must be increasing between zero and kink",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: 2,
                    full_rate: 2,
                    kink_utilization: UTILIZATION_ONE / 2,
                },
                expected: Err(RatesError::ModelNotIncreasingKinkRateAboveFullRate),
                message: "rates must be increasing between kink and 100% util rate",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: 2,
                    full_rate: 3,
                    kink_utilization: UTILIZATION_ONE,
                },
                expected: Err(RatesError::ModelKinkUtilizationOver100Percent),
                message: "kink must be less than 100%",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: 2,
                    full_rate: 3,
                    kink_utilization: 0,
                },
                expected: Err(RatesError::ModelKinkUtilizationNotPositive),
                message: "kink must be more than zero",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: MAX_RATE + 1,
                    kink_rate: 2,
                    full_rate: 3,
                    kink_utilization: 0,
                },
                expected: Err(RatesError::ModelRateOutOfBounds),
                message: "rate must be less than max rate",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: MAX_RATE + 1,
                    full_rate: 3,
                    kink_utilization: 0,
                },
                expected: Err(RatesError::ModelRateOutOfBounds),
                message: "rate must be less than max rate",
            },
            InterestRateModelCheckParametersTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 1,
                    kink_rate: 2,
                    full_rate: MAX_RATE + 1,
                    kink_utilization: 0,
                },
                expected: Err(RatesError::ModelRateOutOfBounds),
                message: "rate must be less than max rate",
            },
        ]
    }

    fn test_check_parameters_case(case: InterestRateModelCheckParametersTestCase) {
        assert_eq!(
            case.model.check_parameters(),
            case.expected,
            "{}",
            case.message
        );
    }

    #[test]
    fn test_check_parameters() {
        get_check_parameters_test_cases()
            .drain(..)
            .for_each(test_check_parameters_case)
    }

    fn get_get_borrow_rate_test_cases() -> Vec<InterestRateModelGetBorrowRateTestCase> {
        // sheet for working out these test cases https://docs.google.com/spreadsheets/d/1s7mASgM2Jlz0sKd7oMRVIujf56QlhiXLQfCg0Tr9dAY/edit?usp=sharing
        vec![
            InterestRateModelGetBorrowRateTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 100,
                    kink_rate: 200,
                    kink_utilization: 5000,
                    full_rate: 500,
                },
                utilization: 0,
                expected: Ok(100),
                message: "rate at zero utilization should be zero utilization rate",
            },
            InterestRateModelGetBorrowRateTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 100,
                    kink_rate: 200,
                    kink_utilization: 5000,
                    full_rate: 500,
                },
                utilization: 5000,
                expected: Ok(200),
                message: "rate at kink utilization should be kink utilization rate",
            },
            InterestRateModelGetBorrowRateTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 100,
                    kink_rate: 200,
                    kink_utilization: 5000,
                    full_rate: 500,
                },
                utilization: UTILIZATION_ONE,
                expected: Ok(500),
                message: "rate at full utilization should be full utilization rate",
            },
            InterestRateModelGetBorrowRateTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 100,
                    kink_rate: 200,
                    kink_utilization: 5000,
                    full_rate: 500,
                },
                utilization: 1000,
                expected: Ok(120),
                message: "rate at point between zero and kink",
            },
            InterestRateModelGetBorrowRateTestCase {
                model: InterestRateModel::Kink {
                    zero_rate: 100,
                    kink_rate: 200,
                    kink_utilization: 5000,
                    full_rate: 500,
                },
                utilization: 8000,
                expected: Ok(380),
                message: "rate at point between kink and full",
            },
        ]
    }

    fn test_get_borrow_rate_case(case: InterestRateModelGetBorrowRateTestCase) {
        assert_eq!(
            case.expected,
            case.model.get_borrow_rate(case.utilization, 0),
            "{}",
            case.message
        )
    }

    #[test]
    fn test_get_borrow_rate() {
        get_get_borrow_rate_test_cases()
            .drain(..)
            .for_each(test_get_borrow_rate_case)
    }
}
