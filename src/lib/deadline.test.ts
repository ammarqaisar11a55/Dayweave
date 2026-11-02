import { describeDeadline } from "./deadline";

// A Monday at 14:00 local time.
const now = new Date(2026, 10, 2, 14, 0);

const open = (dueDate: string | null, dueTime: string | null = null) =>
  describeDeadline({ dueDate, dueTime, status: "todo" }, now);

describe("date-only deadlines", () => {
  it("are described in days and never in hours", () => {
    expect(open("2026-11-02").label).toBe("Today");
    expect(open("2026-11-03").label).toBe("Tomorrow");
    expect(open("2026-11-05").label).toBe("In 3 days");
    expect(open("2026-11-02").detail).toBe("Due today");
    expect(open("2026-11-02").detail).not.toMatch(/hour|minute/);
  });

  it("stay on time until the whole day has passed", () => {
    const lateAtNight = new Date(2026, 10, 2, 23, 59);
    const result = describeDeadline({ dueDate: "2026-11-02", dueTime: null, status: "todo" }, lateAtNight);
    expect(result.tone).toBe("today");
  });

  it("report how many days overdue", () => {
    expect(open("2026-11-01").label).toBe("Overdue by 1 day");
    expect(open("2026-10-30").label).toBe("Overdue by 3 days");
    expect(open("2026-10-30").tone).toBe("overdue");
  });
});

describe("timed deadlines", () => {
  it("count down in minutes, then hours", () => {
    expect(open("2026-11-02", "14:10").detail).toBe("Due in 10 minutes");
    expect(open("2026-11-02", "14:10").label).toBe("In 10 min");
    expect(open("2026-11-02", "16:30").detail).toBe("Due in 2 hours");
    expect(open("2026-11-02", "16:30").label).toBe("Today, 16:30");
    expect(open("2026-11-02", "14:00").label).toBe("Now");
  });

  it("use the day for anything after today", () => {
    expect(open("2026-11-03", "09:00").label).toBe("Tomorrow, 09:00");
    expect(open("2026-11-04", "09:00").label).toBe("In 2 days");
    expect(open("2026-11-04", "09:00").detail).toBe("Due in 2 days, 09:00");
  });

  it("report overdue time in the closest unit", () => {
    expect(open("2026-11-02", "13:59").label).toBe("Overdue by 1 minute");
    expect(open("2026-11-02", "13:30").label).toBe("Overdue by 30 minutes");
    expect(open("2026-11-02", "11:00").label).toBe("Overdue by 3 hours");
    expect(open("2026-10-30", "11:00").label).toBe("Overdue by 3 days");
  });

  it("follow the time format setting", () => {
    const result = describeDeadline({ dueDate: "2026-11-02", dueTime: "16:30", status: "todo" }, now, "12h");
    expect(result.label).toBe("Today, 4:30 PM");
  });
});

describe("other states", () => {
  it("a completed task is never overdue", () => {
    const result = describeDeadline({ dueDate: "2020-01-01", dueTime: null, status: "completed" }, now);
    expect(result).toMatchObject({ label: "Completed", tone: "done" });
  });

  it("a task without a deadline shows nothing", () => {
    expect(open(null)).toMatchObject({ label: "", tone: "none" });
  });

  it("distant deadlines show their date", () => {
    expect(open("2026-12-25").tone).toBe("later");
    expect(open("2026-12-25").label).toMatch(/25/);
  });
});
