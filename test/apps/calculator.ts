const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type Expr = { _tag: "Num"; _0: unknown } | { _tag: "Add"; _0: unknown; _1: unknown } | { _tag: "Mul"; _0: unknown; _1: unknown } | { _tag: "Neg"; _0: unknown };
const Num = (_0: unknown): Expr => ({ _tag: "Num", _0 });
const Add = (_0: unknown, _1: unknown): Expr => ({ _tag: "Add", _0, _1 });
const Mul = (_0: unknown, _1: unknown): Expr => ({ _tag: "Mul", _0, _1 });
const Neg = (_0: unknown): Expr => ({ _tag: "Neg", _0 });

function evaluate(expr) {
  switch (expr._tag) {
    case "Num": {
      const n = expr._0;
      return n;
      break;
    }
    case "Add": {
      const a = expr._0;
      const b = expr._1;
      return evaluate(a) + evaluate(b);
      break;
    }
    case "Mul": {
      const a = expr._0;
      const b = expr._1;
      return evaluate(a) * evaluate(b);
      break;
    }
    case "Neg": {
      const e = expr._0;
      return 0 - evaluate(e);
      break;
    }
  }
}

function show_expr(expr) {
  switch (expr._tag) {
    case "Num": {
      const n = expr._0;
      return show(n);
      break;
    }
    case "Add": {
      const a = expr._0;
      const b = expr._1;
      return "(" + show_expr(a) + " + " + show_expr(b) + ")";
      break;
    }
    case "Mul": {
      const a = expr._0;
      const b = expr._1;
      return "(" + show_expr(a) + " * " + show_expr(b) + ")";
      break;
    }
    case "Neg": {
      const e = expr._0;
      return "(-" + show_expr(e) + ")";
      break;
    }
  }
}

function main() {
  const expr = { _tag: "Add", _0: { _tag: "Mul", _0: { _tag: "Num", _0: 3 }, _1: { _tag: "Num", _0: 4 } }, _1: { _tag: "Neg", _0: { _tag: "Num", _0: 5 } } };
  println("Expression: " + show_expr(expr));
  return println("Result: " + show(evaluate(expr)));
}

main();
