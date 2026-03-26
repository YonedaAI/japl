import { describe, it, expect } from "vitest";
import {
  Ok, Err, Some, None,
  mapResult, flatMapResult, unwrapOr, isOk, isErr,
  mapOption, flatMapOption, unwrapOrOption,
} from "../src/types.js";

describe("Result", () => {
  it("creates Ok values", () => {
    const r = Ok(42);
    expect(r._tag).toBe("Ok");
    expect(r.value).toBe(42);
  });

  it("creates Err values", () => {
    const r = Err("fail");
    expect(r._tag).toBe("Err");
    expect(r.error).toBe("fail");
  });

  it("isOk returns true for Ok", () => {
    expect(isOk(Ok(1))).toBe(true);
    expect(isOk(Err("x"))).toBe(false);
  });

  it("isErr returns true for Err", () => {
    expect(isErr(Err("x"))).toBe(true);
    expect(isErr(Ok(1))).toBe(false);
  });

  it("mapResult transforms Ok values", () => {
    const r = mapResult(Ok(2), x => x * 3);
    expect(r).toEqual(Ok(6));
  });

  it("mapResult passes through Err", () => {
    const r = mapResult(Err("bad"), (x: number) => x * 3);
    expect(r).toEqual(Err("bad"));
  });

  it("flatMapResult chains Ok values", () => {
    const r = flatMapResult(Ok(5), x => x > 3 ? Ok("big") : Err("small"));
    expect(r).toEqual(Ok("big"));
  });

  it("flatMapResult passes through Err", () => {
    const r = flatMapResult(Err("nope") as ReturnType<typeof Err<string>>, (_x: number) => Ok("yes"));
    expect(r).toEqual(Err("nope"));
  });

  it("unwrapOr returns value for Ok", () => {
    expect(unwrapOr(Ok(42), 0)).toBe(42);
  });

  it("unwrapOr returns default for Err", () => {
    expect(unwrapOr(Err("x"), 0)).toBe(0);
  });
});

describe("Option", () => {
  it("creates Some values", () => {
    const o = Some(42);
    expect(o._tag).toBe("Some");
    expect(o.value).toBe(42);
  });

  it("creates None", () => {
    expect(None._tag).toBe("None");
  });

  it("mapOption transforms Some values", () => {
    const o = mapOption(Some(2), x => x * 3);
    expect(o).toEqual(Some(6));
  });

  it("mapOption passes through None", () => {
    const o = mapOption(None, (x: number) => x * 3);
    expect(o).toEqual(None);
  });

  it("flatMapOption chains Some values", () => {
    const o = flatMapOption(Some(5), x => x > 3 ? Some("big") : None);
    expect(o).toEqual(Some("big"));
  });

  it("flatMapOption passes through None", () => {
    const o = flatMapOption(None, (_x: number) => Some("yes"));
    expect(o).toEqual(None);
  });

  it("unwrapOrOption returns value for Some", () => {
    expect(unwrapOrOption(Some(42), 0)).toBe(42);
  });

  it("unwrapOrOption returns default for None", () => {
    expect(unwrapOrOption(None, 0)).toBe(0);
  });
});
