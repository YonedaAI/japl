const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type Json = { _tag: "JNull" } | { _tag: "JBool"; _0: unknown } | { _tag: "JInt"; _0: unknown } | { _tag: "JString"; _0: unknown } | { _tag: "JArray"; _0: unknown; _1: unknown; _2: unknown };
const JNull: Json = { _tag: "JNull" };
const JBool = (_0: unknown): Json => ({ _tag: "JBool", _0 });
const JInt = (_0: unknown): Json => ({ _tag: "JInt", _0 });
const JString = (_0: unknown): Json => ({ _tag: "JString", _0 });
const JArray = (_0: unknown, _1: unknown, _2: unknown): Json => ({ _tag: "JArray", _0, _1, _2 });

function to_string(j) {
  switch (j._tag) {
    case "JNull": {
      return "null";
      break;
    }
    case "JBool": {
      const b = j._0;
      return b ? "true" : "false";
      break;
    }
    case "JInt": {
      const n = j._0;
      return show(n);
      break;
    }
    case "JString": {
      const s = j._0;
      return "\"" + s + "\"";
      break;
    }
    case "JArray": {
      const a = j._0;
      const b = j._1;
      const c = j._2;
      return "[" + to_string(a) + ", " + to_string(b) + ", " + to_string(c) + "]";
      break;
    }
  }
}

function main() {
  const data = { _tag: "JArray", _0: { _tag: "JString", _0: "hello" }, _1: { _tag: "JInt", _0: 42 }, _2: { _tag: "JBool", _0: { _tag: "True" } } };
  println(to_string(data));
  println(to_string({ _tag: "JNull" }));
  return println(to_string({ _tag: "JArray", _0: { _tag: "JInt", _0: 1 }, _1: { _tag: "JInt", _0: 2 }, _2: { _tag: "JInt", _0: 3 } }));
}

main();
