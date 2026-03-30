const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type Tree = { _tag: "Leaf"; _0: unknown } | { _tag: "Branch"; _0: unknown; _1: unknown };
const Leaf = (_0: unknown): Tree => ({ _tag: "Leaf", _0 });
const Branch = (_0: unknown, _1: unknown): Tree => ({ _tag: "Branch", _0, _1 });

function sum_tree(t) {
  switch (t._tag) {
    case "Leaf": {
      const n = t._0;
      return n;
      break;
    }
    case "Branch": {
      const left = t._0;
      const right = t._1;
      return sum_tree(left) + sum_tree(right);
      break;
    }
  }
}

function depth(t) {
  switch (t._tag) {
    case "Leaf": {
      return 1;
      break;
    }
    case "Branch": {
      const left = t._0;
      const right = t._1;
      const ld = depth(left);
      const rd = depth(right);
      return ld > rd ? ld + 1 : rd + 1;
      break;
    }
  }
}

function main() {
  const tree = { _tag: "Branch", _0: { _tag: "Branch", _0: { _tag: "Leaf", _0: 1 }, _1: { _tag: "Leaf", _0: 2 } }, _1: { _tag: "Branch", _0: { _tag: "Leaf", _0: 3 }, _1: { _tag: "Leaf", _0: 4 } } };
  println("Sum: " + show(sum_tree(tree)));
  return println("Depth: " + show(depth(tree)));
}

main();
