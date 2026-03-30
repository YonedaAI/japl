import { describe, it, expect } from "vitest";
import { serialize, ValueTag, serializeByte } from "../../src/wire/serialize.js";
import { deserialize } from "../../src/wire/deserialize.js";

function roundTrip(value: any) {
  const bytes = serialize(value);
  const { value: result, bytesRead } = deserialize(bytes);
  expect(result).toEqual(value);
  expect(bytesRead).toBe(bytes.length);
}

describe("wire codec", () => {
  // Integer
  it("round-trips Int", () => roundTrip(42));
  it("round-trips negative Int", () => roundTrip(-999));
  it("round-trips zero", () => roundTrip(0));
  it("round-trips large Int", () => roundTrip(1_000_000_000));

  // Float
  it("round-trips Float", () => roundTrip(3.14));
  it("round-trips negative Float", () => roundTrip(-0.001));

  // Bool
  it("round-trips Bool true", () => roundTrip(true));
  it("round-trips Bool false", () => roundTrip(false));

  // String
  it("round-trips String", () => roundTrip("hello world"));
  it("round-trips empty String", () => roundTrip(""));
  it("round-trips Unicode String", () => roundTrip("こんにちは"));
  it("round-trips emoji String", () => roundTrip("🎉🚀"));

  // List
  it("round-trips List of Int", () => roundTrip([1, 2, 3]));
  it("round-trips empty List", () => {
    const bytes = serialize([]);
    const { value } = deserialize(bytes);
    expect(value).toEqual([]);
  });
  it("round-trips nested List", () => roundTrip([[1, 2], [3, 4]]));
  it("round-trips mixed List", () => roundTrip([1, "two", true, { _tag: "None" }]));

  // Record
  it("round-trips Record", () => roundTrip({ name: "alice", age: 30 }));
  it("round-trips empty Record", () => roundTrip({}));
  it("round-trips nested Record", () =>
    roundTrip({ user: { name: "bob", scores: [10, 20] } }));

  // Tagged unions
  it("round-trips Tagged (Some)", () => roundTrip({ _tag: "Some", _0: 42 }));
  it("round-trips Tagged (None)", () => roundTrip({ _tag: "None" }));
  it("round-trips Tagged with multiple fields", () =>
    roundTrip({ _tag: "Point", _0: 1.0, _1: 2.0 }));
  it("round-trips Tagged with zero fields", () =>
    roundTrip({ _tag: "Unit" }));
  it("round-trips nested Tagged", () =>
    roundTrip({ _tag: "Ok", _0: { _tag: "Some", _0: "hello" } }));

  // Result
  it("round-trips Result Ok", () => {
    const ok = { _tag: "Ok" as const, value: 42 };
    const bytes = serialize(ok);
    const { value } = deserialize(bytes);
    expect(value).toEqual(ok);
  });
  it("round-trips Result Err", () => {
    const err = { _tag: "Err" as const, error: "not found" };
    const bytes = serialize(err);
    const { value } = deserialize(bytes);
    expect(value).toEqual(err);
  });

  // Option
  it("round-trips Option Some", () => {
    const some = { _tag: "Some" as const, value: "data" };
    const bytes = serialize(some);
    const { value } = deserialize(bytes);
    expect(value).toEqual(some);
  });
  it("round-trips Option None", () => {
    const none = { _tag: "None" as const };
    const bytes = serialize(none);
    const { value } = deserialize(bytes);
    expect(value).toEqual(none);
  });

  // Unit / null
  it("round-trips null as Unit", () => {
    const bytes = serialize(null);
    const { value } = deserialize(bytes);
    expect(value).toBeNull();
  });
  it("round-trips undefined as Unit", () => {
    const bytes = serialize(undefined);
    const { value } = deserialize(bytes);
    expect(value).toBeNull();
  });

  // Byte
  it("round-trips Byte", () => {
    const bytes = serializeByte(0xAB);
    expect(bytes[0]).toBe(ValueTag.BYTE);
    const { value } = deserialize(bytes);
    expect(value).toBe(0xAB);
  });

  // Deeply nested
  it("round-trips deeply nested structure", () =>
    roundTrip({ _tag: "Msg", _0: [{ name: "x", values: [1, 2, 3] }] }));

  // Consistency checks
  it("tag byte is first byte of serialized output", () => {
    expect(serialize(42)[0]).toBe(ValueTag.INT);
    expect(serialize(3.14)[0]).toBe(ValueTag.FLOAT);
    expect(serialize(true)[0]).toBe(ValueTag.BOOL);
    expect(serialize("hi")[0]).toBe(ValueTag.STRING);
    expect(serialize([1])[0]).toBe(ValueTag.LIST);
    expect(serialize([])[0]).toBe(ValueTag.NIL);
    expect(serialize({ a: 1 })[0]).toBe(ValueTag.RECORD);
    expect(serialize({ _tag: "X", _0: 1 })[0]).toBe(ValueTag.TAGGED);
    expect(serialize(null)[0]).toBe(ValueTag.UNIT);
  });

  it("deserialize throws on unknown tag", () => {
    expect(() => deserialize(new Uint8Array([0xFF]))).toThrow("Unknown value tag");
  });
});
