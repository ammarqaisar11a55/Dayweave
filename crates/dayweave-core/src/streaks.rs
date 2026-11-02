//! The streak engine: a pure function of a habit's definition, its history and today's date.
//! The rules are described in ARCHITECTURE.md.

use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Build,
    Quit,
    Quantity,
    Checklist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    /// Due on the selected weekdays, where index 0 is Monday.
    Days([bool; 7]),
    /// Due any days, this many times in each week.
    WeeklyTarget(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Done,
    Partial,
    Slip,
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub kind: Kind,
    pub schedule: Schedule,
    pub start: NaiveDate,
    pub week_start: Weekday,
}

/// How a single day stands for a habit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DayState {
    /// The habit did not exist yet, or this date is still to come.
    Inactive,
    /// Nothing was due on this day.
    NotScheduled,
    Done,
    /// Some progress, but short of the goal.
    Partial,
    Missed,
    /// Due today and not finished yet. Today is never counted as missed.
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StreakUnit {
    Days,
    Weeks,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub current_streak: u32,
    pub best_streak: u32,
    pub streak_unit: StreakUnit,
    /// Share of due occurrences met since the habit started, from 0 to 1.
    pub completion_rate: f64,
    pub weekly_consistency: f64,
    pub monthly_consistency: f64,
    pub total_done: u32,
}

pub struct Engine<'a> {
    spec: &'a Spec,
    entries: &'a BTreeMap<NaiveDate, Entry>,
    today: NaiveDate,
}

impl<'a> Engine<'a> {
    pub fn new(spec: &'a Spec, entries: &'a BTreeMap<NaiveDate, Entry>, today: NaiveDate) -> Self {
        Engine { spec, entries, today }
    }

    fn week_of(&self, date: NaiveDate) -> NaiveDate {
        date.week(self.spec.week_start).first_day()
    }

    /// Whether anything is due on `date`, judged by weekdays alone.
    pub fn is_due(&self, date: NaiveDate) -> bool {
        match self.spec.schedule {
            Schedule::Days(days) => days[date.weekday().num_days_from_monday() as usize],
            Schedule::WeeklyTarget(_) => true,
        }
    }

    fn done_on(&self, date: NaiveDate) -> bool {
        if self.spec.kind == Kind::Quit {
            return !matches!(self.entries.get(&date), Some(Entry::Slip));
        }
        matches!(self.entries.get(&date), Some(Entry::Done))
    }

    fn done_in_week(&self, week: NaiveDate) -> u32 {
        (0..7)
            .map(|offset| week + Duration::days(offset))
            .filter(|date| *date >= self.spec.start && *date <= self.today)
            .filter(|date| self.entries.contains_key(date) && self.done_on(*date))
            .count() as u32
    }

    /// Whether a weekly target is still open on `date`, so today asks for a completion.
    fn target_open(&self, date: NaiveDate) -> bool {
        match self.spec.schedule {
            Schedule::WeeklyTarget(target) => self.done_in_week(self.week_of(date)) < target,
            Schedule::Days(_) => false,
        }
    }

    pub fn state(&self, date: NaiveDate) -> DayState {
        if date < self.spec.start || date > self.today {
            return DayState::Inactive;
        }
        let entry = self.entries.get(&date);
        if let Schedule::WeeklyTarget(_) = self.spec.schedule {
            return match entry {
                Some(Entry::Done) => DayState::Done,
                Some(Entry::Partial) => DayState::Partial,
                _ if date == self.today && self.target_open(date) => DayState::Pending,
                _ => DayState::NotScheduled,
            };
        }
        if !self.is_due(date) {
            return match entry {
                Some(Entry::Done) if self.spec.kind != Kind::Quit => DayState::Done,
                Some(Entry::Slip) => DayState::Missed,
                _ => DayState::NotScheduled,
            };
        }
        match (self.spec.kind, entry) {
            (Kind::Quit, Some(Entry::Slip)) => DayState::Missed,
            (Kind::Quit, _) if date == self.today => DayState::Pending,
            (Kind::Quit, _) => DayState::Done,
            (_, Some(Entry::Done)) => DayState::Done,
            (_, Some(Entry::Partial)) if date == self.today => DayState::Pending,
            (_, Some(Entry::Partial)) => DayState::Partial,
            _ if date == self.today => DayState::Pending,
            _ => DayState::Missed,
        }
    }

    pub fn stats(&self) -> Stats {
        match self.spec.schedule {
            Schedule::Days(_) => self.daily_stats(),
            Schedule::WeeklyTarget(target) => self.weekly_stats(target),
        }
    }

    fn days(&self) -> impl Iterator<Item = NaiveDate> + '_ {
        self.spec.start.iter_days().take_while(|date| *date <= self.today)
    }

    fn daily_stats(&self) -> Stats {
        let (mut run, mut best, mut done, mut due) = (0u32, 0u32, 0u32, 0u32);
        for date in self.days() {
            if !self.is_due(date) {
                // An unscheduled slip still breaks a quit habit.
                if self.spec.kind == Kind::Quit && !self.done_on(date) {
                    run = 0;
                }
                continue;
            }
            match self.state(date) {
                DayState::Done => {
                    run += 1;
                    best = best.max(run);
                    done += 1;
                    due += 1;
                }
                DayState::Pending => {}
                _ => {
                    run = 0;
                    due += 1;
                }
            }
        }
        Stats {
            current_streak: run,
            best_streak: best,
            streak_unit: StreakUnit::Days,
            completion_rate: ratio(done, due),
            weekly_consistency: self.window_rate(7),
            monthly_consistency: self.window_rate(30),
            total_done: done,
        }
    }

    fn window_rate(&self, days: i64) -> f64 {
        let (mut done, mut due) = (0, 0);
        for offset in 0..days {
            match self.state(self.today - Duration::days(offset)) {
                DayState::Done if self.is_due(self.today - Duration::days(offset)) => {
                    done += 1;
                    due += 1;
                }
                DayState::Missed | DayState::Partial => due += 1,
                _ => {}
            }
        }
        ratio(done, due)
    }

    fn weekly_stats(&self, target: u32) -> Stats {
        let first = self.week_of(self.spec.start);
        let current = self.week_of(self.today);
        let (mut run, mut best, mut met, mut counted) = (0u32, 0u32, 0u32, 0u32);
        let mut week = first;
        while week <= current {
            let done = self.done_in_week(week);
            if done >= target {
                run += 1;
                best = best.max(run);
                met += 1;
                counted += 1;
            } else if week == current {
                // The week is still going, so it can neither break nor extend the streak.
            } else if week == first && self.spec.start > first {
                // The habit began part-way through, so that week could not fairly be met.
            } else {
                run = 0;
                counted += 1;
            }
            week += Duration::weeks(1);
        }
        let total_done = self.entries.values().filter(|entry| **entry == Entry::Done).count() as u32;
        Stats {
            current_streak: run,
            best_streak: best,
            streak_unit: StreakUnit::Weeks,
            completion_rate: ratio(met, counted),
            weekly_consistency: self.target_rate(7, target),
            monthly_consistency: self.target_rate(30, target),
            total_done,
        }
    }

    fn target_rate(&self, days: i64, target: u32) -> f64 {
        let span = (self.today - self.spec.start).num_days().min(days - 1) + 1;
        if span <= 0 {
            return 0.0;
        }
        let expected = (f64::from(target) * span as f64 / 7.0).max(1.0);
        let done = (0..span)
            .map(|offset| self.today - Duration::days(offset))
            .filter(|date| matches!(self.entries.get(date), Some(Entry::Done)))
            .count();
        (done as f64 / expected).min(1.0)
    }
}

fn ratio(done: u32, due: u32) -> f64 {
    if due == 0 {
        0.0
    } else {
        f64::from(done) / f64::from(due)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    const EVERY_DAY: Schedule = Schedule::Days([true; 7]);
    // Monday, Wednesday and Friday.
    const MWF: Schedule = Schedule::Days([true, false, true, false, true, false, false]);

    fn spec(kind: Kind, schedule: Schedule, start: &str) -> Spec {
        Spec { kind, schedule, start: date(start), week_start: Weekday::Mon }
    }

    fn entries(done: &[&str]) -> BTreeMap<NaiveDate, Entry> {
        done.iter().map(|day| (date(day), Entry::Done)).collect()
    }

    // 2026-11-02 is a Monday.
    #[test]
    fn consecutive_days_build_a_streak() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-10-28");
        let log = entries(&["2026-10-29", "2026-10-30", "2026-10-31", "2026-11-01", "2026-11-02"]);
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        assert_eq!(stats.current_streak, 5);
        assert_eq!(stats.best_streak, 5);
        assert_eq!(stats.total_done, 5);
    }

    #[test]
    fn a_missed_day_resets_the_current_streak_and_keeps_the_best() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-10-26");
        let log = entries(&["2026-10-26", "2026-10-27", "2026-10-28", "2026-10-30", "2026-10-31"]);
        let stats = Engine::new(&spec, &log, date("2026-10-31")).stats();
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.best_streak, 3);
    }

    #[test]
    fn today_being_unfinished_does_not_break_a_streak() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-10-30");
        let log = entries(&["2026-10-30", "2026-10-31", "2026-11-01"]);
        let engine = Engine::new(&spec, &log, date("2026-11-02"));
        assert_eq!(engine.stats().current_streak, 3);
        assert_eq!(engine.state(date("2026-11-02")), DayState::Pending);
    }

    #[test]
    fn missing_yesterday_breaks_a_streak_even_while_today_is_pending() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-10-29");
        let log = entries(&["2026-10-29", "2026-10-30", "2026-10-31"]);
        assert_eq!(Engine::new(&spec, &log, date("2026-11-02")).stats().current_streak, 0);
    }

    #[test]
    fn unscheduled_days_neither_extend_nor_break_a_streak() {
        let spec = spec(Kind::Build, MWF, "2026-10-26");
        let log = entries(&["2026-10-26", "2026-10-28", "2026-10-30", "2026-11-02"]);
        let stats = Engine::new(&spec, &log, date("2026-11-03")).stats();
        assert_eq!(stats.current_streak, 4);
    }

    #[test]
    fn a_bonus_completion_on_an_unscheduled_day_does_not_count_towards_the_streak() {
        let spec = spec(Kind::Build, MWF, "2026-10-26");
        let log = entries(&["2026-10-26", "2026-10-27", "2026-10-28"]);
        let engine = Engine::new(&spec, &log, date("2026-10-28"));
        assert_eq!(engine.stats().current_streak, 2);
        assert_eq!(engine.state(date("2026-10-27")), DayState::Done);
    }

    #[test]
    fn a_scheduled_day_missed_after_an_unscheduled_gap_breaks_the_streak() {
        let spec = spec(Kind::Build, MWF, "2026-10-26");
        let log = entries(&["2026-10-26", "2026-10-30"]);
        assert_eq!(Engine::new(&spec, &log, date("2026-10-31")).stats().current_streak, 1);
    }

    #[test]
    fn days_before_the_start_date_never_count() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-11-01");
        let log = entries(&["2026-10-30", "2026-10-31", "2026-11-01"]);
        let engine = Engine::new(&spec, &log, date("2026-11-01"));
        assert_eq!(engine.stats().current_streak, 1);
        assert_eq!(engine.state(date("2026-10-31")), DayState::Inactive);
    }

    #[test]
    fn a_new_habit_starts_empty() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-11-02");
        let log = BTreeMap::new();
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.best_streak, 0);
        assert_eq!(stats.completion_rate, 0.0);
    }

    #[test]
    fn a_habit_starting_in_the_future_has_no_active_days() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-12-01");
        let log = BTreeMap::new();
        let engine = Engine::new(&spec, &log, date("2026-11-02"));
        assert_eq!(engine.state(date("2026-11-02")), DayState::Inactive);
        assert_eq!(engine.stats().completion_rate, 0.0);
    }

    #[test]
    fn completion_rate_counts_only_due_days_and_leaves_out_a_pending_today() {
        let spec = spec(Kind::Build, MWF, "2026-10-26");
        let log = entries(&["2026-10-26", "2026-10-30"]);
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        assert!((stats.completion_rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn partial_progress_is_visible_but_breaks_a_streak() {
        let spec = spec(Kind::Quantity, EVERY_DAY, "2026-10-30");
        let mut log = entries(&["2026-10-30", "2026-11-01"]);
        log.insert(date("2026-10-31"), Entry::Partial);
        let engine = Engine::new(&spec, &log, date("2026-11-01"));
        assert_eq!(engine.state(date("2026-10-31")), DayState::Partial);
        assert_eq!(engine.stats().current_streak, 1);
    }

    #[test]
    fn partial_progress_today_stays_pending() {
        let spec = spec(Kind::Quantity, EVERY_DAY, "2026-11-01");
        let log = BTreeMap::from([(date("2026-11-01"), Entry::Done), (date("2026-11-02"), Entry::Partial)]);
        let engine = Engine::new(&spec, &log, date("2026-11-02"));
        assert_eq!(engine.state(date("2026-11-02")), DayState::Pending);
        assert_eq!(engine.stats().current_streak, 1);
    }

    #[test]
    fn quit_habits_succeed_each_day_unless_a_slip_is_logged() {
        let spec = spec(Kind::Quit, EVERY_DAY, "2026-10-28");
        let log = BTreeMap::new();
        let engine = Engine::new(&spec, &log, date("2026-11-02"));
        assert_eq!(engine.stats().current_streak, 5);
        assert_eq!(engine.state(date("2026-11-01")), DayState::Done);
        assert_eq!(engine.state(date("2026-11-02")), DayState::Pending);
    }

    #[test]
    fn a_slip_restarts_a_quit_streak() {
        let spec = spec(Kind::Quit, EVERY_DAY, "2026-10-26");
        let log = BTreeMap::from([(date("2026-10-30"), Entry::Slip)]);
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        // The 31st and the 1st. Today has not finished, so it does not count yet.
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.best_streak, 4);
    }

    #[test]
    fn a_slip_today_ends_the_streak_immediately() {
        let spec = spec(Kind::Quit, EVERY_DAY, "2026-10-30");
        let log = BTreeMap::from([(date("2026-11-02"), Entry::Slip)]);
        let engine = Engine::new(&spec, &log, date("2026-11-02"));
        assert_eq!(engine.stats().current_streak, 0);
        assert_eq!(engine.state(date("2026-11-02")), DayState::Missed);
    }

    #[test]
    fn a_slip_on_an_unscheduled_day_still_breaks_a_quit_streak() {
        let spec = spec(Kind::Quit, MWF, "2026-10-26");
        let log = BTreeMap::from([(date("2026-10-29"), Entry::Slip)]);
        let stats = Engine::new(&spec, &log, date("2026-11-03")).stats();
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.best_streak, 2);
    }

    #[test]
    fn weekly_targets_are_measured_in_weeks() {
        let spec = spec(Kind::Build, Schedule::WeeklyTarget(2), "2026-10-19");
        let log = entries(&["2026-10-19", "2026-10-21", "2026-10-26", "2026-10-30"]);
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.streak_unit, StreakUnit::Weeks);
    }

    #[test]
    fn a_week_short_of_its_target_breaks_the_streak() {
        let spec = spec(Kind::Build, Schedule::WeeklyTarget(2), "2026-10-12");
        let log = entries(&["2026-10-12", "2026-10-13", "2026-10-20", "2026-10-26", "2026-10-27"]);
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        assert_eq!(stats.current_streak, 1);
        assert_eq!(stats.best_streak, 1);
    }

    #[test]
    fn the_current_week_can_extend_but_not_break_a_weekly_streak() {
        let spec = spec(Kind::Build, Schedule::WeeklyTarget(2), "2026-10-26");
        let log = entries(&["2026-10-26", "2026-10-28"]);
        let engine = Engine::new(&spec, &log, date("2026-11-02"));
        assert_eq!(engine.stats().current_streak, 1);
        assert_eq!(engine.state(date("2026-11-02")), DayState::Pending);
    }

    #[test]
    fn meeting_a_weekly_target_clears_the_rest_of_that_week() {
        let spec = spec(Kind::Build, Schedule::WeeklyTarget(1), "2026-11-02");
        let log = entries(&["2026-11-02"]);
        let engine = Engine::new(&spec, &log, date("2026-11-03"));
        assert_eq!(engine.state(date("2026-11-03")), DayState::NotScheduled);
    }

    #[test]
    fn a_habit_starting_mid_week_is_not_penalised_for_that_week() {
        let spec = spec(Kind::Build, Schedule::WeeklyTarget(3), "2026-10-29");
        let log = entries(&["2026-11-02", "2026-11-03", "2026-11-04"]);
        let stats = Engine::new(&spec, &log, date("2026-11-08")).stats();
        assert_eq!(stats.current_streak, 1);
        assert_eq!(stats.completion_rate, 1.0);
    }

    #[test]
    fn the_week_start_setting_changes_how_weeks_are_grouped() {
        // Sunday and Monday fall in one week when weeks begin on Sunday, but not when they begin on Monday.
        let log = entries(&["2026-11-01", "2026-11-02"]);
        let mut spec = spec(Kind::Build, Schedule::WeeklyTarget(2), "2026-10-26");
        assert_eq!(Engine::new(&spec, &log, date("2026-11-07")).stats().current_streak, 0);
        spec.week_start = Weekday::Sun;
        assert_eq!(Engine::new(&spec, &log, date("2026-11-07")).stats().current_streak, 1);
    }

    #[test]
    fn consistency_uses_trailing_windows() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-09-01");
        let log = entries(&["2026-11-02", "2026-11-01", "2026-10-31", "2026-10-30"]);
        let stats = Engine::new(&spec, &log, date("2026-11-02")).stats();
        assert!((stats.weekly_consistency - 4.0 / 7.0).abs() < 1e-9);
        assert!((stats.monthly_consistency - 4.0 / 30.0).abs() < 1e-9);
    }

    #[test]
    fn a_future_date_is_inactive() {
        let spec = spec(Kind::Build, EVERY_DAY, "2026-11-01");
        let log = BTreeMap::new();
        assert_eq!(
            Engine::new(&spec, &log, date("2026-11-02")).state(date("2026-11-03")),
            DayState::Inactive
        );
    }
}
