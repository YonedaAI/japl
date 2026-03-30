export enum ValueTag {
  INT        = 0x01,
  FLOAT      = 0x02,
  BOOL       = 0x03,
  STRING     = 0x04,
  BYTE       = 0x05,
  LIST       = 0x06,
  RECORD     = 0x07,
  TAGGED     = 0x08,
  PID        = 0x09,
  UNIT       = 0x0A,
  NIL        = 0x0B,
  TUPLE      = 0x0C,
  RESULT_OK  = 0x0D,
  RESULT_ERR = 0x0E,
  OPTION_SOME = 0x0F,
  OPTION_NONE = 0x10,
}

const encoder = new TextEncoder();

export function serialize(value: any): Uint8Array {
  if (value === undefined || value === null) return serializeUnit();
  if (typeof value === "number") {
    if (Number.isInteger(value)) return serializeInt(value);
    return serializeFloat(value);
  }
  if (typeof value === "boolean") return serializeBool(value);
  if (typeof value === "string") return serializeString(value);
  if (Array.isArray(value)) return serializeList(value);
  if (typeof value === "object") {
    if ("_tag" in value) {
      const tag = value._tag as string;
      // Result/Option types use .value/.error, tagged unions use ._0, ._1, ...
      if (tag === "Ok" && "value" in value) return serializeResultOk(value.value);
      if (tag === "Err" && "error" in value) return serializeResultErr(value.error);
      if (tag === "Some" && "value" in value) return serializeOptionSome(value.value);
      if (tag === "None" && !("_0" in value)) return serializeOptionNone();
      return serializeTagged(value);
    }
    return serializeRecord(value);
  }
  throw new Error(`Cannot serialize: ${typeof value}`);
}

function serializeInt(n: number): Uint8Array {
  const buf = new Uint8Array(9);
  buf[0] = ValueTag.INT;
  new DataView(buf.buffer).setBigInt64(1, BigInt(n));
  return buf;
}

function serializeFloat(n: number): Uint8Array {
  const buf = new Uint8Array(9);
  buf[0] = ValueTag.FLOAT;
  new DataView(buf.buffer).setFloat64(1, n);
  return buf;
}

function serializeBool(b: boolean): Uint8Array {
  return new Uint8Array([ValueTag.BOOL, b ? 1 : 0]);
}

function serializeString(s: string): Uint8Array {
  const encoded = encoder.encode(s);
  const buf = new Uint8Array(1 + 4 + encoded.length);
  buf[0] = ValueTag.STRING;
  new DataView(buf.buffer).setUint32(1, encoded.length);
  buf.set(encoded, 5);
  return buf;
}

/** Encode a raw string (no tag byte) for use as record keys and tagged names. */
function serializeStringRaw(s: string): Uint8Array {
  const encoded = encoder.encode(s);
  const buf = new Uint8Array(4 + encoded.length);
  new DataView(buf.buffer).setUint32(0, encoded.length);
  buf.set(encoded, 4);
  return buf;
}

export function serializeByte(b: number): Uint8Array {
  return new Uint8Array([ValueTag.BYTE, b & 0xff]);
}

function serializeList(arr: any[]): Uint8Array {
  if (arr.length === 0) {
    return new Uint8Array([ValueTag.NIL]);
  }
  const parts = arr.map(serialize);
  const totalPayload = parts.reduce((sum, p) => sum + p.length, 0);
  const buf = new Uint8Array(1 + 4 + totalPayload);
  buf[0] = ValueTag.LIST;
  new DataView(buf.buffer).setUint32(1, arr.length);
  let offset = 5;
  for (const part of parts) {
    buf.set(part, offset);
    offset += part.length;
  }
  return buf;
}

function serializeRecord(obj: Record<string, any>): Uint8Array {
  const entries = Object.entries(obj);
  const parts: Uint8Array[] = [];
  for (const [key, val] of entries) {
    parts.push(serializeStringRaw(key));
    parts.push(serialize(val));
  }
  const totalPayload = parts.reduce((sum, p) => sum + p.length, 0);
  const buf = new Uint8Array(1 + 4 + totalPayload);
  buf[0] = ValueTag.RECORD;
  new DataView(buf.buffer).setUint32(1, entries.length);
  let offset = 5;
  for (const part of parts) {
    buf.set(part, offset);
    offset += part.length;
  }
  return buf;
}

function serializeTagged(obj: any): Uint8Array {
  const tag = obj._tag as string;
  const fields: any[] = [];
  let i = 0;
  while (`_${i}` in obj) {
    fields.push(obj[`_${i}`]);
    i++;
  }
  const tagBytes = serializeStringRaw(tag);
  const fieldParts = fields.map(serialize);
  const fieldPayload = fieldParts.reduce((sum, p) => sum + p.length, 0);
  const buf = new Uint8Array(1 + tagBytes.length + 4 + fieldPayload);
  buf[0] = ValueTag.TAGGED;
  buf.set(tagBytes, 1);
  const fieldCountOffset = 1 + tagBytes.length;
  new DataView(buf.buffer).setUint32(fieldCountOffset, fields.length);
  let offset = fieldCountOffset + 4;
  for (const part of fieldParts) {
    buf.set(part, offset);
    offset += part.length;
  }
  return buf;
}

function serializeResultOk(value: any): Uint8Array {
  const inner = serialize(value);
  const buf = new Uint8Array(1 + inner.length);
  buf[0] = ValueTag.RESULT_OK;
  buf.set(inner, 1);
  return buf;
}

function serializeResultErr(error: any): Uint8Array {
  const inner = serialize(error);
  const buf = new Uint8Array(1 + inner.length);
  buf[0] = ValueTag.RESULT_ERR;
  buf.set(inner, 1);
  return buf;
}

function serializeOptionSome(value: any): Uint8Array {
  const inner = serialize(value);
  const buf = new Uint8Array(1 + inner.length);
  buf[0] = ValueTag.OPTION_SOME;
  buf.set(inner, 1);
  return buf;
}

function serializeOptionNone(): Uint8Array {
  return new Uint8Array([ValueTag.OPTION_NONE]);
}

function serializeUnit(): Uint8Array {
  return new Uint8Array([ValueTag.UNIT]);
}
