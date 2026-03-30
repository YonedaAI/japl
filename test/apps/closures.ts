const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

function make_adder(n) {
  return (x) => x + n;
}

function main() {
  const add5 = make_adder(5);
  const add10 = make_adder(10);
  println(show(add5(3)));
  println(show(add10(3)));
  return println(show(add5(add10(1))));
}

main();
