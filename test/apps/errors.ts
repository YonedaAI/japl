const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type MyResult = { _tag: "MyOk"; _0: unknown } | { _tag: "MyErr"; _0: unknown };
const MyOk = (_0: unknown): MyResult => ({ _tag: "MyOk", _0 });
const MyErr = (_0: unknown): MyResult => ({ _tag: "MyErr", _0 });

function divide(a, b) {
  return b === 0 ? { _tag: "MyErr", _0: "division by zero" } : { _tag: "MyOk", _0: a / b };
}

function main() {
  switch (divide(10, 2)._tag) {
    case "MyOk": {
      const n = divide(10, 2)._0;
      println("10/2 = " + show(n));
      break;
    }
    case "MyErr": {
      const e = divide(10, 2)._0;
      println("Error: " + e);
      break;
    }
  }
  switch (divide(10, 0)._tag) {
    case "MyOk": {
      const n = divide(10, 0)._0;
      return println("10/0 = " + show(n));
      break;
    }
    case "MyErr": {
      const e = divide(10, 0)._0;
      return println("Error: " + e);
      break;
    }
  }
}

main();
