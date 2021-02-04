use frame_support::sp_runtime::traits::Convert;
use frame_support::sp_std;

/// A `Convert` implementation for Moment -> Timestamp
pub struct TimeConverter<T>(sp_std::marker::PhantomData<T>);

/// It is very sad that this is the way but I can not conceive of a more concise way of doing this.
impl<T: pallet_timestamp::Config> Convert<T::Moment, crate::types::Timestamp> for TimeConverter<T>
where
    T: pallet_timestamp::Config<Moment = crate::types::Timestamp>,
{
    fn convert(source: T::Moment) -> crate::types::Timestamp {
        source as crate::types::Timestamp
    }
}
