export type ProcessId = string;

export type Result<T, E> =
  | { _tag: "Ok"; value: T }
  | { _tag: "Err"; error: E };

export type Option<T> =
  | { _tag: "Some"; value: T }
  | { _tag: "None" };

export const Ok = <T>(value: T): Result<T, never> => ({ _tag: "Ok", value });
export const Err = <E>(error: E): Result<never, E> => ({ _tag: "Err", error });
export const Some = <T>(value: T): Option<T> => ({ _tag: "Some", value });
export const None: Option<never> = { _tag: "None" };

// Monadic operations for Result
export function mapResult<T, U, E>(r: Result<T, E>, f: (t: T) => U): Result<U, E> {
  return r._tag === "Ok" ? Ok(f(r.value)) : r;
}

export function flatMapResult<T, U, E>(r: Result<T, E>, f: (t: T) => Result<U, E>): Result<U, E> {
  return r._tag === "Ok" ? f(r.value) : r;
}

export function unwrapOr<T, E>(r: Result<T, E>, def: T): T {
  return r._tag === "Ok" ? r.value : def;
}

export function isOk<T, E>(r: Result<T, E>): r is { _tag: "Ok"; value: T } {
  return r._tag === "Ok";
}

export function isErr<T, E>(r: Result<T, E>): r is { _tag: "Err"; error: E } {
  return r._tag === "Err";
}

// Monadic operations for Option
export function mapOption<T, U>(o: Option<T>, f: (t: T) => U): Option<U> {
  return o._tag === "Some" ? Some(f(o.value)) : o;
}

export function flatMapOption<T, U>(o: Option<T>, f: (t: T) => Option<U>): Option<U> {
  return o._tag === "Some" ? f(o.value) : o;
}

export function unwrapOrOption<T>(o: Option<T>, def: T): T {
  return o._tag === "Some" ? o.value : def;
}
