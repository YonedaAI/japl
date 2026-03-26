// Runtime support for the JAPL self-hosted compiler
// These functions are used by the compiled TypeScript output

import * as fs from 'node:fs';

// List operations
export function cons(x: unknown, xs: unknown[]): unknown[] {
  return [x, ...xs];
}

export function append(xs: unknown[], ys: unknown[]): unknown[] {
  return [...xs, ...ys];
}

// String operations
export function string_to_chars(s: string): string[] {
  return Array.from(s);
}

export function char_at(s: string, i: number): string {
  return s[i] ?? '';
}

export function string_length(s: string): number {
  return s.length;
}

export function substring(s: string, start: number, end: number): string {
  return s.slice(start, end);
}

// IO operations
export function read_file(filepath: string): string {
  return fs.readFileSync(filepath, 'utf-8');
}

export function get_arg(n: number): string {
  return process.argv[n + 1] ?? '';
}

export function println(s: string): void {
  console.log(s);
}

// Conversion
export function show(x: unknown): string {
  return String(x);
}
