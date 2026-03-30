const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

function double(x) {
  return x * 2;
}

function add(x, y) {
  return x + y;
}

function to_str(x) {
  return show(x);
}

function main() {
  const result = double(double(5));
  println(show(result));
  const msg = to_str(42);
  return println(msg);
}

main();
