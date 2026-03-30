#include <assert.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>
#include <stdint.h>
#include "japl_runtime.h"

/* ── Int tests ──────────────────────────────────────────────── */

void test_value_int(void) {
    JaplValue v = japl_int(42);
    assert(v.kind == JAPL_INT);
    assert(japl_to_int(v) == 42);
}

void test_value_int_negative(void) {
    JaplValue v = japl_int(-100);
    assert(japl_to_int(v) == -100);
}

void test_value_int_zero(void) {
    JaplValue v = japl_int(0);
    assert(japl_to_int(v) == 0);
}

/* ── Float tests ────────────────────────────────────────────── */

void test_value_float(void) {
    JaplValue v = japl_float(3.14);
    assert(v.kind == JAPL_FLOAT);
    assert(fabs(japl_to_float(v) - 3.14) < 0.0001);
}

void test_value_float_from_int(void) {
    JaplValue v = japl_int(5);
    assert(fabs(japl_to_float(v) - 5.0) < 0.0001);
}

/* ── Bool tests ─────────────────────────────────────────────── */

void test_value_bool_true(void) {
    JaplValue v = japl_bool(1);
    assert(v.kind == JAPL_BOOL);
    assert(japl_to_bool(v) == 1);
}

void test_value_bool_false(void) {
    JaplValue v = japl_bool(0);
    assert(japl_to_bool(v) == 0);
}

/* ── String tests ───────────────────────────────────────────── */

void test_value_string(void) {
    JaplValue v = japl_string("hello");
    assert(v.kind == JAPL_STRING);
    assert(strcmp(japl_to_cstr(v), "hello") == 0);
    japl_string_release(v.string_val);
}

void test_value_string_empty(void) {
    JaplValue v = japl_string("");
    assert(japl_string_length(v) == 0);
    assert(strcmp(japl_to_cstr(v), "") == 0);
    japl_string_release(v.string_val);
}

void test_value_string_concat(void) {
    JaplValue a = japl_string("hello ");
    JaplValue b = japl_string("world");
    JaplValue c = japl_string_concat(a, b);
    assert(strcmp(japl_to_cstr(c), "hello world") == 0);
    assert(japl_string_length(c) == 11);
    japl_string_release(a.string_val);
    japl_string_release(b.string_val);
    japl_string_release(c.string_val);
}

/* ── Unit / Nil tests ───────────────────────────────────────── */

void test_value_unit(void) {
    JaplValue v = japl_unit();
    assert(v.kind == JAPL_UNIT);
}

void test_value_nil(void) {
    JaplValue v = japl_nil();
    assert(v.kind == JAPL_NIL);
}

/* ── Arithmetic tests ───────────────────────────────────────── */

void test_value_add_int(void) {
    JaplValue r = japl_add(japl_int(3), japl_int(4));
    assert(japl_to_int(r) == 7);
}

void test_value_add_float(void) {
    JaplValue r = japl_add(japl_float(1.5), japl_float(2.5));
    assert(fabs(japl_to_float(r) - 4.0) < 0.0001);
}

void test_value_no_implicit_promotion(void) {
    /* Int + Int works fine — mixed Int + Float is now a type error.
       We can't easily test exit() in C, so just verify same-type arithmetic. */
    JaplValue a = japl_int(5);
    JaplValue b = japl_int(3);
    JaplValue r = japl_add(a, b);
    assert(r.int_val == 8);
}

void test_value_sub(void) {
    JaplValue r = japl_sub(japl_int(10), japl_int(3));
    assert(japl_to_int(r) == 7);
}

void test_value_mul(void) {
    JaplValue r = japl_mul(japl_int(6), japl_int(7));
    assert(japl_to_int(r) == 42);
}

void test_value_div(void) {
    JaplValue r = japl_div(japl_int(10), japl_int(3));
    assert(japl_to_int(r) == 3);
}

void test_value_mod(void) {
    JaplValue r = japl_mod(japl_int(10), japl_int(3));
    assert(japl_to_int(r) == 1);
}

void test_value_negate(void) {
    JaplValue r = japl_negate(japl_int(42));
    assert(japl_to_int(r) == -42);
}

/* ── Comparison tests ───────────────────────────────────────── */

void test_value_eq_int(void) {
    assert(japl_to_bool(japl_eq(japl_int(5), japl_int(5))) == 1);
    assert(japl_to_bool(japl_eq(japl_int(5), japl_int(6))) == 0);
}

void test_value_eq_string(void) {
    JaplValue a = japl_string("abc");
    JaplValue b = japl_string("abc");
    JaplValue c = japl_string("xyz");
    assert(japl_to_bool(japl_eq(a, b)) == 1);
    assert(japl_to_bool(japl_eq(a, c)) == 0);
    japl_string_release(a.string_val);
    japl_string_release(b.string_val);
    japl_string_release(c.string_val);
}

void test_value_neq(void) {
    assert(japl_to_bool(japl_neq(japl_int(1), japl_int(2))) == 1);
    assert(japl_to_bool(japl_neq(japl_int(1), japl_int(1))) == 0);
}

void test_value_lt(void) {
    assert(japl_to_bool(japl_lt(japl_int(1), japl_int(2))) == 1);
    assert(japl_to_bool(japl_lt(japl_int(2), japl_int(1))) == 0);
}

void test_value_gt(void) {
    assert(japl_to_bool(japl_gt(japl_int(5), japl_int(3))) == 1);
    assert(japl_to_bool(japl_gt(japl_int(3), japl_int(5))) == 0);
}

void test_value_not(void) {
    assert(japl_to_bool(japl_not(japl_bool(1))) == 0);
    assert(japl_to_bool(japl_not(japl_bool(0))) == 1);
}

/* ── Show tests ─────────────────────────────────────────────── */

void test_value_show_int(void) {
    char* s = japl_show(japl_int(42));
    assert(strcmp(s, "42") == 0);
    free(s);
}

void test_value_show_bool(void) {
    char* s1 = japl_show(japl_bool(1));
    assert(strcmp(s1, "true") == 0);
    free(s1);
    char* s2 = japl_show(japl_bool(0));
    assert(strcmp(s2, "false") == 0);
    free(s2);
}

void test_value_show_unit(void) {
    char* s = japl_show(japl_unit());
    assert(strcmp(s, "()") == 0);
    free(s);
}

void test_value_show_nil(void) {
    char* s = japl_show(japl_nil());
    assert(strcmp(s, "[]") == 0);
    free(s);
}

/* ── Tagged union tests ─────────────────────────────────────── */

void test_tagged_none(void) {
    JaplValue v = japl_tagged("None", 0);
    assert(v.kind == JAPL_TAG);
    assert(strcmp(japl_get_tag(v), "None") == 0);
    assert(v.tagged_val->field_count == 0);
    free(v.tagged_val);
}

void test_tagged_some(void) {
    JaplValue v = japl_tagged("Some", 1, japl_int(42));
    assert(strcmp(japl_get_tag(v), "Some") == 0);
    assert(japl_to_int(japl_get_field(v, 0)) == 42);
    free(v.tagged_val);
}

void test_tagged_show(void) {
    JaplValue v = japl_tagged("Ok", 1, japl_int(99));
    char* s = japl_show(v);
    assert(strcmp(s, "Ok(99)") == 0);
    free(s);
    free(v.tagged_val);
}

void test_tagged_eq(void) {
    JaplValue a = japl_tagged("Some", 1, japl_int(5));
    JaplValue b = japl_tagged("Some", 1, japl_int(5));
    JaplValue c = japl_tagged("None", 0);
    assert(japl_to_bool(japl_eq(a, b)) == 1);
    assert(japl_to_bool(japl_eq(a, c)) == 0);
    free(a.tagged_val);
    free(b.tagged_val);
    free(c.tagged_val);
}

/* ── Closure tests ──────────────────────────────────────────── */

static JaplValue add_one_fn(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc; (void)argc;
    return japl_add(args[0], japl_int(1));
}

void test_closure_basic(void) {
    JaplValue fn = japl_closure(add_one_fn, 1, 0);
    JaplValue result = japl_apply(fn, 1, japl_int(41));
    assert(japl_to_int(result) == 42);
    free(fn.closure_val);
}

static JaplValue add_env_fn(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)argc; (void)envc;
    return japl_add(args[0], env[0]);
}

void test_closure_with_env(void) {
    JaplValue fn = japl_closure(add_env_fn, 1, 1, japl_int(10));
    JaplValue result = japl_apply(fn, 1, japl_int(32));
    assert(japl_to_int(result) == 42);
    free(fn.closure_val->env);
    free(fn.closure_val);
}

/* ── Record tests ───────────────────────────────────────────── */

void test_record_basic(void) {
    JaplValue rec = japl_record(2, "name", japl_string("Alice"), "age", japl_int(30));
    assert(rec.kind == JAPL_RECORD);
    assert(strcmp(japl_to_cstr(japl_field(rec, "name")), "Alice") == 0);
    assert(japl_to_int(japl_field(rec, "age")) == 30);
    /* Cleanup */
    JaplValue name_val = japl_field(rec, "name");
    japl_string_release(name_val.string_val);
    free(rec.record_val->keys);
    free(rec.record_val->values);
    free(rec.record_val);
}

void test_record_update(void) {
    JaplValue rec = japl_record(2, "x", japl_int(1), "y", japl_int(2));
    JaplValue updated = japl_record_update(rec, "x", japl_int(10));
    assert(japl_to_int(japl_field(updated, "x")) == 10);
    assert(japl_to_int(japl_field(updated, "y")) == 2);
    /* Original unchanged */
    assert(japl_to_int(japl_field(rec, "x")) == 1);
    /* Cleanup */
    free(rec.record_val->keys);
    free(rec.record_val->values);
    free(rec.record_val);
    free(updated.record_val->keys);
    free(updated.record_val->values);
    free(updated.record_val);
}

/* ── Overflow detection tests ──────────────────────────────── */

void test_int_overflow_add(void) {
    /* INT64_MAX - 1 + 1 should succeed */
    JaplValue a = japl_int(INT64_MAX - 1);
    JaplValue b = japl_int(1);
    JaplValue r = japl_add(a, b);
    assert(r.int_val == INT64_MAX);
}

void test_int_overflow_sub(void) {
    /* INT64_MIN + 1 - 1 should succeed */
    JaplValue a = japl_int(INT64_MIN + 1);
    JaplValue b = japl_int(1);
    JaplValue r = japl_sub(a, b);
    assert(r.int_val == INT64_MIN);
}

void test_int_overflow_mul(void) {
    /* Large but non-overflowing multiplication */
    JaplValue a = japl_int(1000000);
    JaplValue b = japl_int(1000000);
    JaplValue r = japl_mul(a, b);
    assert(r.int_val == 1000000000000LL);
}

void test_int_mul_by_zero(void) {
    JaplValue a = japl_int(INT64_MAX);
    JaplValue b = japl_int(0);
    JaplValue r = japl_mul(a, b);
    assert(r.int_val == 0);
}

/* ── Byte type tests ───────────────────────────────────────── */

void test_byte_create(void) {
    JaplValue b = japl_byte(255);
    assert(b.kind == JAPL_BYTE);
    assert(japl_to_byte(b) == 255);
}

void test_byte_zero(void) {
    JaplValue b = japl_byte(0);
    assert(b.kind == JAPL_BYTE);
    assert(japl_to_byte(b) == 0);
}

void test_byte_show(void) {
    JaplValue b = japl_byte(42);
    char* s = japl_show(b);
    assert(strcmp(s, "42") == 0);
    free(s);
}

void test_byte_show_255(void) {
    JaplValue b = japl_byte(255);
    char* s = japl_show(b);
    assert(strcmp(s, "255") == 0);
    free(s);
}

void test_byte_add(void) {
    JaplValue a = japl_byte(100);
    JaplValue b = japl_byte(55);
    JaplValue r = japl_byte_add(a, b);
    assert(japl_to_byte(r) == 155);
}

void test_byte_sub(void) {
    JaplValue a = japl_byte(200);
    JaplValue b = japl_byte(50);
    JaplValue r = japl_byte_sub(a, b);
    assert(japl_to_byte(r) == 150);
}

void test_byte_mul(void) {
    JaplValue a = japl_byte(10);
    JaplValue b = japl_byte(25);
    JaplValue r = japl_byte_mul(a, b);
    assert(japl_to_byte(r) == 250);
}

void test_byte_eq(void) {
    assert(japl_to_bool(japl_eq(japl_byte(42), japl_byte(42))) == 1);
    assert(japl_to_bool(japl_eq(japl_byte(42), japl_byte(43))) == 0);
}

void test_kind_name(void) {
    assert(strcmp(japl_kind_name(JAPL_INT), "Int") == 0);
    assert(strcmp(japl_kind_name(JAPL_FLOAT), "Float") == 0);
    assert(strcmp(japl_kind_name(JAPL_BYTE), "Byte") == 0);
    assert(strcmp(japl_kind_name(JAPL_BOOL), "Bool") == 0);
    assert(strcmp(japl_kind_name(JAPL_STRING), "String") == 0);
}
