const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type User = {
  name: string;
  age: number;
  active: boolean;
};

function main() {
  const user = { name: "Alice", age: 30, active: true };
  println(user.name);
  println(show(user.age));
  const updated = { ...user, age: 31, active: false };
  println(show(updated.age));
  return println(show(updated.active));
}

main();
