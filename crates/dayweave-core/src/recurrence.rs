//! Works out when a recurring task happens next. Pure date arithmetic, with no storage.

use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Frequency {
    pub fn as_str(self) -> &'static str {
        match self {
            Frequency::Daily => "daily",
            Frequency::Weekly => "weekly",
            Frequency::Monthly => "monthly",
            Frequency::Yearly => "yearly",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "daily" => Some(Frequency::Daily),
            "weekly" => Some(Frequency::Weekly),
            "monthly" => Some(Frequency::Monthly),
            "yearly" => Some(Frequency::Yearly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub frequency: Frequency,
    pub interval: u32,
    /// Bitmask of weekdays for weekly rules, where bit 0 is Monday. Zero means the anchor's weekday.
    pub weekdays: u8,
    /// The day of the month a monthly or yearly rule aims for, kept through shorter months.
    pub anchor_day: Option<u32>,
    pub end_date: Option<NaiveDate>,
    pub max_count: Option<u32>,
}

impl Rule {
    pub fn simple(frequency: Frequency) -> Self {
        Rule { frequency, interval: 1, weekdays: 0, anchor_day: None, end_date: None, max_count: None }
    }

    /// The occurrence straight after `from`, which must itself be an occurrence.
    fn step(&self, from: NaiveDate) -> NaiveDate {
        let interval = self.interval.max(1);
        match self.frequency {
            Frequency::Daily => from + Duration::days(i64::from(interval)),
            Frequency::Weekly if self.weekdays == 0 => from + Duration::weeks(i64::from(interval)),
            Frequency::Weekly => self.next_weekday(from, interval),
            Frequency::Monthly => self.in_month(from, interval),
            Frequency::Yearly => self.in_month(from, interval * 12),
        }
    }

    fn next_weekday(&self, from: NaiveDate, interval: u32) -> NaiveDate {
        let week_start = |date: NaiveDate| date.week(Weekday::Mon).first_day();
        let anchor = week_start(from);
        let mut candidate = from + Duration::days(1);
        // Only weeks an exact number of intervals from the anchor week are eligible.
        for _ in 0..(7 * (interval as usize + 1)) {
            let weeks = (week_start(candidate) - anchor).num_weeks();
            let selected = self.weekdays & (1 << candidate.weekday().num_days_from_monday()) != 0;
            if selected && weeks % i64::from(interval) == 0 {
                return candidate;
            }
            candidate += Duration::days(1);
        }
        from + Duration::weeks(i64::from(interval))
    }

    fn in_month(&self, from: NaiveDate, months: u32) -> NaiveDate {
        let day = self.anchor_day.unwrap_or_else(|| from.day());
        let first = from.with_day(1).expect("every month has a first day") + Months::new(months);
        let last = (first + Months::new(1)).pred_opt().expect("not the minimum date").day();
        first.with_day(day.min(last)).expect("clamped to the length of the month")
    }

    /// When the next occurrence falls, given the one just finished. Occurrences that passed while
    /// the task sat overdue are skipped. `None` means the series has ended.
    pub fn next_occurrence(&self, due: NaiveDate, today: NaiveDate, created: u32) -> Option<NaiveDate> {
        if self.max_count.is_some_and(|limit| created >= limit) {
            return None;
        }
        let mut next = self.step(due);
        while next <= today {
            next = self.step(next);
        }
        match self.end_date {
            Some(end) if next > end => None,
            _ => Some(next),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// Follows a rule for `count` occurrences, completing each on the day it falls due.
    fn series(rule: &Rule, start: &str, count: usize) -> Vec<String> {
        let mut current = date(start);
        let mut result = Vec::new();
        for _ in 0..count {
            current = rule.next_occurrence(current, current, 0).unwrap();
            result.push(current.format("%Y-%m-%d").to_string());
        }
        result
    }

    const MONDAY: u8 = 1;
    const WEDNESDAY: u8 = 1 << 2;
    const FRIDAY: u8 = 1 << 4;

    #[test]
    fn daily_rules_repeat_at_their_interval() {
        let rule = Rule { interval: 3, ..Rule::simple(Frequency::Daily) };
        assert_eq!(series(&rule, "2026-11-30", 2), ["2026-12-03", "2026-12-06"]);
    }

    #[test]
    fn weekly_rules_keep_the_original_weekday() {
        let rule = Rule::simple(Frequency::Weekly);
        assert_eq!(series(&rule, "2026-11-02", 2), ["2026-11-09", "2026-11-16"]);
    }

    #[test]
    fn weekly_rules_visit_each_selected_weekday() {
        let rule = Rule { weekdays: MONDAY | WEDNESDAY | FRIDAY, ..Rule::simple(Frequency::Weekly) };
        assert_eq!(series(&rule, "2026-11-02", 4), ["2026-11-04", "2026-11-06", "2026-11-09", "2026-11-11"]);
    }

    #[test]
    fn fortnightly_rules_skip_the_weeks_between() {
        let rule = Rule { interval: 2, weekdays: MONDAY | FRIDAY, ..Rule::simple(Frequency::Weekly) };
        assert_eq!(series(&rule, "2026-11-02", 3), ["2026-11-06", "2026-11-16", "2026-11-20"]);
    }

    #[test]
    fn monthly_rules_clamp_to_short_months_and_return_to_the_anchor() {
        let rule = Rule { anchor_day: Some(31), ..Rule::simple(Frequency::Monthly) };
        assert_eq!(series(&rule, "2027-01-31", 3), ["2027-02-28", "2027-03-31", "2027-04-30"]);
    }

    #[test]
    fn monthly_rules_use_a_leap_day_when_there_is_one() {
        let rule = Rule { anchor_day: Some(30), ..Rule::simple(Frequency::Monthly) };
        assert_eq!(series(&rule, "2028-01-30", 2), ["2028-02-29", "2028-03-30"]);
    }

    #[test]
    fn yearly_rules_on_a_leap_day_clamp_and_return() {
        let rule = Rule { anchor_day: Some(29), ..Rule::simple(Frequency::Yearly) };
        assert_eq!(series(&rule, "2028-02-29", 4), ["2029-02-28", "2030-02-28", "2031-02-28", "2032-02-29"]);
    }

    #[test]
    fn monthly_rules_cross_the_year_end() {
        let rule = Rule { interval: 2, ..Rule::simple(Frequency::Monthly) };
        assert_eq!(series(&rule, "2026-11-15", 2), ["2027-01-15", "2027-03-15"]);
    }

    #[test]
    fn occurrences_missed_while_overdue_are_skipped() {
        let rule = Rule::simple(Frequency::Daily);
        let next = rule.next_occurrence(date("2026-10-20"), date("2026-11-02"), 1);
        assert_eq!(next, Some(date("2026-11-03")));
    }

    #[test]
    fn a_skipped_weekly_rule_stays_on_its_weekday() {
        let rule = Rule::simple(Frequency::Weekly);
        let next = rule.next_occurrence(date("2026-10-05"), date("2026-11-02"), 1);
        assert_eq!(next, Some(date("2026-11-09")));
    }

    #[test]
    fn completing_early_keeps_the_schedule() {
        let rule = Rule::simple(Frequency::Weekly);
        let next = rule.next_occurrence(date("2026-11-13"), date("2026-11-02"), 1);
        assert_eq!(next, Some(date("2026-11-20")));
    }

    #[test]
    fn a_series_stops_at_its_end_date() {
        let rule = Rule { end_date: Some(date("2026-11-05")), ..Rule::simple(Frequency::Daily) };
        assert_eq!(rule.next_occurrence(date("2026-11-04"), date("2026-11-04"), 1), Some(date("2026-11-05")));
        assert_eq!(rule.next_occurrence(date("2026-11-05"), date("2026-11-05"), 2), None);
    }

    #[test]
    fn a_series_stops_at_its_occurrence_limit() {
        let rule = Rule { max_count: Some(3), ..Rule::simple(Frequency::Daily) };
        assert!(rule.next_occurrence(date("2026-11-02"), date("2026-11-02"), 2).is_some());
        assert_eq!(rule.next_occurrence(date("2026-11-03"), date("2026-11-03"), 3), None);
    }
}
