import {
  addDays,
  addMonths,
  daysBetween,
  formatClock,
  formatDuration,
  formatTime,
  greeting,
  monthGrid,
  parseDate,
  startOfWeek,
  toDateText,
  weekDates,
} from "./date";

describe("calendar dates", () => {
  it("round-trips without shifting a day", () => {
    expect(toDateText(parseDate("2026-11-02"))).toBe("2026-11-02");
    expect(toDateText(parseDate("2026-01-01"))).toBe("2026-01-01");
  });

  it("moves across month and year ends", () => {
    expect(addDays("2026-12-31", 1)).toBe("2027-01-01");
    expect(addDays("2026-03-01", -1)).toBe("2026-02-28");
  });

  it("clamps to the length of the target month", () => {
    expect(addMonths("2027-01-31", 1)).toBe("2027-02-28");
    expect(addMonths("2026-12-15", 1)).toBe("2027-01-15");
    expect(addMonths("2027-01-15", -1)).toBe("2026-12-15");
  });

  it("counts whole days", () => {
    expect(daysBetween("2026-11-02", "2026-11-05")).toBe(3);
    expect(daysBetween("2026-11-05", "2026-11-02")).toBe(-3);
    expect(daysBetween("2026-03-01", "2026-04-01")).toBe(31);
  });
});

describe("weeks", () => {
  // 2026-11-04 is a Wednesday.
  it("honours the week start setting", () => {
    expect(startOfWeek("2026-11-04", "monday")).toBe("2026-11-02");
    expect(startOfWeek("2026-11-04", "sunday")).toBe("2026-11-01");
    expect(startOfWeek("2026-11-04", "saturday")).toBe("2026-10-31");
    expect(startOfWeek("2026-11-02", "monday")).toBe("2026-11-02");
  });

  it("lists seven days", () => {
    expect(weekDates("2026-11-04", "monday")).toHaveLength(7);
    expect(weekDates("2026-11-04", "monday")[6]).toBe("2026-11-08");
  });

  it("covers a whole month in full weeks", () => {
    const grid = monthGrid("2026-11-15", "monday");
    expect(grid[0]?.[0]).toBe("2026-10-26");
    expect(grid[grid.length - 1]?.[6]).toBe("2026-12-06");
    expect(grid.every((row) => row.length === 7)).toBe(true);
  });

  it("uses only as many rows as the month needs", () => {
    // February 2027 starts on a Monday and has exactly four weeks.
    expect(monthGrid("2027-02-10", "monday")).toHaveLength(4);
  });
});

describe("formatting", () => {
  it("converts to a twelve-hour clock", () => {
    expect(formatTime("00:05", "12h")).toBe("12:05 AM");
    expect(formatTime("12:00", "12h")).toBe("12:00 PM");
    expect(formatTime("17:30", "12h")).toBe("5:30 PM");
    expect(formatTime("17:30", "24h")).toBe("17:30");
  });

  it("describes durations", () => {
    expect(formatDuration(1500)).toBe("25 min");
    expect(formatDuration(3600)).toBe("1 h");
    expect(formatDuration(5400)).toBe("1 h 30 min");
  });

  it("formats a countdown", () => {
    expect(formatClock(1500)).toBe("25:00");
    expect(formatClock(59)).toBe("00:59");
    expect(formatClock(3661)).toBe("1:01:01");
    expect(formatClock(-4)).toBe("00:00");
  });

  it("greets by the time of day", () => {
    expect(greeting(3)).toBe("Good night");
    expect(greeting(9)).toBe("Good morning");
    expect(greeting(14)).toBe("Good afternoon");
    expect(greeting(20)).toBe("Good evening");
  });
});
