//! Every conversion between local wall-clock values and instants goes through this module, so
//! daylight saving is handled in exactly one place.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat};
use chrono::{TimeZone, Utc};

use crate::error::{Error, Result};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn to_local(&self, instant: DateTime<Utc>) -> NaiveDateTime;
    fn to_instant(&self, wall: NaiveDateTime) -> DateTime<Utc>;

    fn local_now(&self) -> NaiveDateTime {
        self.to_local(self.now())
    }

    fn today(&self) -> NaiveDate {
        self.local_now().date()
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn to_local(&self, instant: DateTime<Utc>) -> NaiveDateTime {
        instant.with_timezone(&Local).naive_local()
    }

    fn to_instant(&self, wall: NaiveDateTime) -> DateTime<Utc> {
        resolve(&Local, wall)
    }
}

/// A clock that only moves when told to, in any timezone. Used by tests and the development bridge.
pub struct ManualClock<Tz: TimeZone> {
    now: Mutex<DateTime<Utc>>,
    tz: Tz,
}

impl<Tz: TimeZone> ManualClock<Tz> {
    pub fn new(tz: Tz, wall: &str) -> Self {
        let wall = parse_datetime(wall).expect("valid wall-clock time");
        ManualClock { now: Mutex::new(resolve(&tz, wall)), tz }
    }

    pub fn set(&self, wall: &str) {
        let wall = parse_datetime(wall).expect("valid wall-clock time");
        *self.now.lock().unwrap() = resolve(&self.tz, wall);
    }

    pub fn advance(&self, duration: Duration) {
        *self.now.lock().unwrap() += duration;
    }
}

impl<Tz> Clock for ManualClock<Tz>
where
    Tz: TimeZone + Send + Sync,
    Tz::Offset: Send + Sync,
{
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }

    fn to_local(&self, instant: DateTime<Utc>) -> NaiveDateTime {
        instant.with_timezone(&self.tz).naive_local()
    }

    fn to_instant(&self, wall: NaiveDateTime) -> DateTime<Utc> {
        resolve(&self.tz, wall)
    }
}

/// Resolves a wall-clock value in `tz`. A time skipped by a daylight saving change moves forward to
/// the first valid minute, and a repeated time resolves to its first occurrence.
pub fn resolve<Tz: TimeZone>(tz: &Tz, wall: NaiveDateTime) -> DateTime<Utc> {
    let mut candidate = wall;
    // The largest real-world gap is an hour, except for rare whole-day zone changes.
    for _ in 0..(26 * 60) {
        match tz.from_local_datetime(&candidate) {
            LocalResult::Single(time) | LocalResult::Ambiguous(time, _) => {
                return time.with_timezone(&Utc);
            }
            LocalResult::None => candidate += Duration::minutes(1),
        }
    }
    Utc.from_utc_datetime(&wall)
}

pub fn format_instant(instant: DateTime<Utc>) -> String {
    instant.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn parse_instant(text: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| Error::invalid(format!("'{text}' is not a valid timestamp")))
}

pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn parse_date(text: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| Error::invalid(format!("'{text}' is not a valid date")))
}

pub fn format_time(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

pub fn parse_time(text: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(text, "%H:%M")
        .map_err(|_| Error::invalid(format!("'{text}' is not a valid time")))
}

pub fn format_datetime(wall: NaiveDateTime) -> String {
    wall.format("%Y-%m-%dT%H:%M").to_string()
}

pub fn parse_datetime(text: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M")
        .map_err(|_| Error::invalid(format!("'{text}' is not a valid date and time")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::New_York;
    use chrono_tz::Asia::Karachi;

    fn wall(text: &str) -> NaiveDateTime {
        parse_datetime(text).unwrap()
    }

    #[test]
    fn a_time_skipped_by_daylight_saving_moves_to_the_first_valid_minute() {
        // Clocks in New York jumped from 02:00 to 03:00 on 2026-03-08.
        let instant = resolve(&New_York, wall("2026-03-08T02:30"));
        assert_eq!(format_instant(instant), "2026-03-08T07:00:00Z");
        assert_eq!(instant.with_timezone(&New_York).format("%H:%M").to_string(), "03:00");
    }

    #[test]
    fn a_repeated_time_resolves_to_its_first_occurrence() {
        // Clocks in New York fell back from 02:00 to 01:00 on 2026-11-01.
        let instant = resolve(&New_York, wall("2026-11-01T01:30"));
        assert_eq!(format_instant(instant), "2026-11-01T05:30:00Z");
    }

    #[test]
    fn a_fixed_wall_clock_time_keeps_its_local_hour_across_a_change() {
        let before = resolve(&New_York, wall("2026-10-31T09:00"));
        let after = resolve(&New_York, wall("2026-11-02T09:00"));
        assert_eq!(after - before, Duration::hours(49));
    }

    #[test]
    fn zones_without_daylight_saving_convert_directly() {
        let instant = resolve(&Karachi, wall("2026-11-02T17:00"));
        assert_eq!(format_instant(instant), "2026-11-02T12:00:00Z");
    }

    #[test]
    fn the_local_date_comes_from_the_clock_zone_rather_than_utc() {
        let clock = ManualClock::new(Karachi, "2026-11-03T02:00");
        assert_eq!(format_instant(clock.now()), "2026-11-02T21:00:00Z");
        assert_eq!(format_date(clock.today()), "2026-11-03");
    }

    #[test]
    fn malformed_values_are_rejected_with_a_readable_message() {
        assert!(parse_date("2026-13-40").unwrap_err().to_string().contains("not a valid date"));
        assert!(parse_time("25:00").is_err());
        assert!(parse_instant("yesterday").is_err());
    }
}
