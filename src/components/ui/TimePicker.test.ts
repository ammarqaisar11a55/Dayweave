import { parseTimeInput } from "./TimePicker";

describe("parseTimeInput", () => {
  it("understands the ways people type a time", () => {
    expect(parseTimeInput("9")).toBe("09:00");
    expect(parseTimeInput("9:30")).toBe("09:30");
    expect(parseTimeInput("0930")).toBe("09:30");
    expect(parseTimeInput("17")).toBe("17:00");
    expect(parseTimeInput("5pm")).toBe("17:00");
    expect(parseTimeInput("5:30 PM")).toBe("17:30");
    expect(parseTimeInput("12am")).toBe("00:00");
    expect(parseTimeInput("12pm")).toBe("12:00");
  });

  it("rejects what isn't a time", () => {
    expect(parseTimeInput("25")).toBeNull();
    expect(parseTimeInput("9:75")).toBeNull();
    expect(parseTimeInput("13pm")).toBeNull();
    expect(parseTimeInput("soon")).toBeNull();
  });
});
