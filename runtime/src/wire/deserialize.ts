import { ValueTag } from "./serialize.js";

const decoder = new TextDecoder();

export interface DeserializeResult {
  value: any;
  bytesRead: number;
}

function readStringRaw(buf: Uint8Array, offset: number): { value: string; bytesRead: number } {
  const view = new DataView(buf.buffer, buf.byteOffset + offset);
  const len = view.getUint32(0);
  const str = decoder.decode(buf.subarray(offset + 4, offset + 4 + len));
  return { value: str, bytesRead: 4 + len };
}

export function deserialize(buf: Uint8Array, offset = 0): DeserializeResult {
  const tag = buf[offset];

  switch (tag) {
    case ValueTag.INT: {
      const view = new DataView(buf.buffer, buf.byteOffset + offset + 1);
      return { value: Number(view.getBigInt64(0)), bytesRead: 9 };
    }

    case ValueTag.FLOAT: {
      const view = new DataView(buf.buffer, buf.byteOffset + offset + 1);
      return { value: view.getFloat64(0), bytesRead: 9 };
    }

    case ValueTag.BOOL:
      return { value: buf[offset + 1] === 1, bytesRead: 2 };

    case ValueTag.STRING: {
      const view = new DataView(buf.buffer, buf.byteOffset + offset + 1);
      const len = view.getUint32(0);
      const str = decoder.decode(buf.subarray(offset + 5, offset + 5 + len));
      return { value: str, bytesRead: 5 + len };
    }

    case ValueTag.BYTE:
      return { value: buf[offset + 1], bytesRead: 2 };

    case ValueTag.LIST: {
      const view = new DataView(buf.buffer, buf.byteOffset + offset + 1);
      const count = view.getUint32(0);
      const elements: any[] = [];
      let pos = offset + 5;
      for (let i = 0; i < count; i++) {
        const { value, bytesRead } = deserialize(buf, pos);
        elements.push(value);
        pos += bytesRead;
      }
      return { value: elements, bytesRead: pos - offset };
    }

    case ValueTag.RECORD: {
      const view = new DataView(buf.buffer, buf.byteOffset + offset + 1);
      const fieldCount = view.getUint32(0);
      const obj: Record<string, any> = {};
      let pos = offset + 5;
      for (let i = 0; i < fieldCount; i++) {
        const keyResult = readStringRaw(buf, pos);
        pos += keyResult.bytesRead;
        const valResult = deserialize(buf, pos);
        pos += valResult.bytesRead;
        obj[keyResult.value] = valResult.value;
      }
      return { value: obj, bytesRead: pos - offset };
    }

    case ValueTag.TAGGED: {
      const tagStr = readStringRaw(buf, offset + 1);
      let pos = offset + 1 + tagStr.bytesRead;
      const view = new DataView(buf.buffer, buf.byteOffset + pos);
      const fieldCount = view.getUint32(0);
      pos += 4;
      const result: any = { _tag: tagStr.value };
      for (let i = 0; i < fieldCount; i++) {
        const { value, bytesRead } = deserialize(buf, pos);
        result[`_${i}`] = value;
        pos += bytesRead;
      }
      return { value: result, bytesRead: pos - offset };
    }

    case ValueTag.PID: {
      const nodeResult = readStringRaw(buf, offset + 1);
      let pos = offset + 1 + nodeResult.bytesRead;
      const idResult = readStringRaw(buf, pos);
      pos += idResult.bytesRead;
      return {
        value: { node: nodeResult.value, id: idResult.value },
        bytesRead: pos - offset,
      };
    }

    case ValueTag.UNIT:
      return { value: null, bytesRead: 1 };

    case ValueTag.NIL:
      return { value: [], bytesRead: 1 };

    case ValueTag.TUPLE: {
      const view = new DataView(buf.buffer, buf.byteOffset + offset + 1);
      const count = view.getUint32(0);
      const elements: any[] = [];
      let pos = offset + 5;
      for (let i = 0; i < count; i++) {
        const { value, bytesRead } = deserialize(buf, pos);
        elements.push(value);
        pos += bytesRead;
      }
      return { value: elements, bytesRead: pos - offset };
    }

    case ValueTag.RESULT_OK: {
      const { value, bytesRead } = deserialize(buf, offset + 1);
      return {
        value: { _tag: "Ok", value },
        bytesRead: 1 + bytesRead,
      };
    }

    case ValueTag.RESULT_ERR: {
      const { value, bytesRead } = deserialize(buf, offset + 1);
      return {
        value: { _tag: "Err", error: value },
        bytesRead: 1 + bytesRead,
      };
    }

    case ValueTag.OPTION_SOME: {
      const { value, bytesRead } = deserialize(buf, offset + 1);
      return {
        value: { _tag: "Some", value },
        bytesRead: 1 + bytesRead,
      };
    }

    case ValueTag.OPTION_NONE:
      return { value: { _tag: "None" }, bytesRead: 1 };

    default:
      throw new Error(`Unknown value tag: 0x${tag.toString(16).padStart(2, "0")}`);
  }
}
