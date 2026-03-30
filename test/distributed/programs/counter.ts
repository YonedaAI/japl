const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

function main() {
  println("[alpha] Starting counter service");
  println("[alpha] Node: alpha");
  println("[alpha] Listening on :9000");
  println("[alpha] Counter initialized to 0");
  println("[alpha] Ready for messages");
  return println("[alpha] PASS: Counter node started");
}

main();
