#[cfg(feature = "runtime-benchmarks")]

use pallet_cash::{Module as Cash, frame_system};
pub struct Module<T: Config>(pallet_cash::Module<T>);

mod benchmarking {
    use frame_benchmarking::{benchmarks, account, impl_benchmark_test_suite};
    use crate::{*, Module as PalletModule};

    benchmarks!{
        cull_notices {

        }:_ Cash::cull_notices(RawOrigin::Root)
    }

    impl_benchmark_test_suite! {
	Module,
	crate::mock::new_test_ext(),
	crate::mock::Test,
	extra = false,
    }
}