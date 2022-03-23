use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ========================================================================= //

/// Converts a `SystemTime` to a Windows `FILETIME`.
pub fn filetime_from_system_time(timestamp: SystemTime) -> u64 {
    match timestamp.duration_since(filetime_epoch()) {
        Ok(delta) => {
            delta.as_secs() * 10_000_000 + (delta.subsec_nanos() / 100) as u64
        }
        Err(_) => 0,
    }
}

/// Converts a Windows `FILETIME` to a `SystemTime`.
pub fn system_time_from_filetime(timestamp: u64) -> SystemTime {
    let delta = Duration::new(
        timestamp / 10_000_000,
        (timestamp % 10_000_000) as u32 * 100,
    );
    filetime_epoch() + delta
}

/// Returns the epoch used for Windows `FILETIME` values.
fn filetime_epoch() -> SystemTime {
    // The epoch used by CFB files is Jan 1, 1601 UTC, which we can calculate
    // from the Unix epoch constant, which is Jan 1, 1970 UTC.
    UNIX_EPOCH - Duration::from_secs(11644473600)
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{filetime_from_system_time, system_time_from_filetime};
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn to_system_time() {
        let sat_2017_mar_18_at_18_46_36_gmt =
            UNIX_EPOCH + Duration::from_secs(1489862796);
        assert_eq!(
            system_time_from_filetime(131343363960000000),
            sat_2017_mar_18_at_18_46_36_gmt
        );
    }

    #[test]
    fn from_system_time() {
        let sat_2017_mar_18_at_18_46_36_gmt =
            UNIX_EPOCH + Duration::from_secs(1489862796);
        assert_eq!(
            filetime_from_system_time(sat_2017_mar_18_at_18_46_36_gmt),
            131343363960000000
        );
    }
}

// ========================================================================= //
