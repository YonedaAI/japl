import { readFileSync, writeFileSync } from 'node:fs';

const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;



function count_chars(s) {
  return string_length(s);
}

function banner(s) {
  return "=== " + s + " ===";
}

function main() {
  const line1 = "Hello from JAPL!";
  const line2 = "This is a real program.";
  const line3 = "It processes files.";
  writeFileSync("/tmp/japl_input.txt", line1);
  const content = readFileSync("/tmp/japl_input.txt", "utf-8");
  const chars = count_chars(content);
  println("File Report:");
  println("Characters: " + show(chars));
  println("Content: " + banner(content));
  const output = "Processed: " + banner(line1) + " | " + banner(line2) + " | " + banner(line3);
  writeFileSync("/tmp/japl_output.txt", output);
  println(output);
  return println("Files written successfully.");
}

main();
