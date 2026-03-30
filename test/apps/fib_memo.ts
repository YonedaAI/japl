const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type Pair = { _tag: "MkPair"; _0: unknown; _1: unknown };
const MkPair = (_0: unknown, _1: unknown): Pair => ({ _tag: "MkPair", _0, _1 });

function fib_pair(n) {
  if (n <= 0) {
    return { _tag: "MkPair", _0: 0, _1: 1 };
  } else {
    const prev = fib_pair(n - 1);
    switch (prev._tag) {
      case "MkPair": {
        const a = prev._0;
        const b = prev._1;
        return { _tag: "MkPair", _0: b, _1: a + b };
        break;
      }
    }
  }
}

function fib(n) {
  switch (fib_pair(n)._tag) {
    case "MkPair": {
      const result = fib_pair(n)._0;
      return result;
      break;
    }
  }
}

function main() {
  println("Fibonacci sequence:");
  println("fib(0) = " + show(fib(0)));
  println("fib(1) = " + show(fib(1)));
  println("fib(5) = " + show(fib(5)));
  println("fib(10) = " + show(fib(10)));
  println("fib(20) = " + show(fib(20)));
  return println("fib(30) = " + show(fib(30)));
}

main();
