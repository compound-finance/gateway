use sp_std;

/// Type for representing time since current Unix epoch in milliseconds.
pub type Timestamp = u64;

/// A `Convert` implementation for Moment -> Timestamp
pub struct TimeConverter<T>(sp_std::marker::PhantomData<T>);

pub trait GetConvertedTimestamp<M> {
    fn get_recent_timestamp() -> Result<Timestamp, String>;
}

impl<T: pallet_timestamp::Config> GetConvertedTimestamp<T::Moment> for TimeConverter<T>
where
    T: pallet_timestamp::Config<Moment = Timestamp>,
{
    /// Return the recent timestamp (from the timestamp pallet).
    #[cfg(not(all(feature = "freeze-time", feature = "std")))]
    fn get_recent_timestamp() -> Result<Timestamp, String> {
        let ts = <pallet_timestamp::Pallet<T>>::get();
        let time = ts as Timestamp;
        if time > 0 {
            return Ok(time);
        } else {
            return Err("Missing Timestamp".to_string());
        }
    }

    /// Return the recent timestamp (from the FREEZE_TIME file).
    #[cfg(all(feature = "freeze-time", feature = "std"))]
    pub fn get_recent_timestamp<T: Config>() -> Result<Timestamp, String> {
        use std::{env, fs};
        if let Ok(filename) = env::var("FREEZE_TIME") {
            if let Ok(contents) = fs::read_to_string(filename) {
                if let Ok(time) = contents.parse::<u64>() {
                    println!("Freeze Time: {}", time);
                    if time > 0 {
                        return Ok(time);
                    }
                }
            }
        }
        return Err("Missing Timestamp".to_string());
    }
}
