const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

function apply(f, x) {
  return f(x);
}

function double(x) {
  return x * 2;
}

function square(x) {
  return x * x;
}

function apply_twice(f, x) {
  return f(f(x));
}

function main() {
  println(show(apply(double, 5)));
  println(show(apply(square, 4)));
  println(show(apply_twice(double, 3)));
  const add_ten = (x) => x + 10;
  return println(show(apply(add_ten, 5)));
}

main();
