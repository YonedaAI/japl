#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <inttypes.h>
#include "japl_value.h"
#include "japl_string.h"
#include "japl_list.h"

/* ── Constructors ───────────────────────────────────────────── */

JaplValue japl_int(int64_t v) {
    JaplValue val;
    val.kind = JAPL_INT;
    val.int_val = v;
    return val;
}

JaplValue japl_float(double v) {
    JaplValue val;
    val.kind = JAPL_FLOAT;
    val.float_val = v;
    return val;
}

JaplValue japl_bool(int v) {
    JaplValue val;
    val.kind = JAPL_BOOL;
    val.bool_val = v ? 1 : 0;
    return val;
}

JaplValue japl_unit(void) {
    JaplValue val;
    val.kind = JAPL_UNIT;
    val.int_val = 0;
    return val;
}

JaplValue japl_nil(void) {
    JaplValue val;
    val.kind = JAPL_NIL;
    val.int_val = 0;
    return val;
}

JaplValue japl_pid(uint64_t pid) {
    JaplValue val;
    val.kind = JAPL_PID;
    val.pid_val = pid;
    return val;
}

/* ── Tagged unions ──────────────────────────────────────────── */

JaplValue japl_tagged(const char* tag, int count, ...) {
    JaplTagged* t = malloc(sizeof(JaplTagged));
    if (!t) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    t->tag = tag;
    t->field_count = count > JAPL_TAG_MAX_FIELDS ? JAPL_TAG_MAX_FIELDS : count;

    va_list ap;
    va_start(ap, count);
    for (int i = 0; i < t->field_count; i++) {
        t->fields[i] = va_arg(ap, JaplValue);
    }
    va_end(ap);

    JaplValue val;
    val.kind = JAPL_TAG;
    val.tagged_val = t;
    return val;
}

JaplValue japl_tagged_v(const char* tag, int count, JaplValue* fields) {
    JaplTagged* t = malloc(sizeof(JaplTagged));
    if (!t) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    t->tag = tag;
    t->field_count = count > JAPL_TAG_MAX_FIELDS ? JAPL_TAG_MAX_FIELDS : count;
    for (int i = 0; i < t->field_count; i++) {
        t->fields[i] = fields[i];
    }

    JaplValue val;
    val.kind = JAPL_TAG;
    val.tagged_val = t;
    return val;
}

const char* japl_get_tag(JaplValue v) {
    if (v.kind != JAPL_TAG) {
        fprintf(stderr, "japl: get_tag on non-tagged value\n");
        abort();
    }
    return v.tagged_val->tag;
}

JaplValue japl_get_field(JaplValue v, int index) {
    if (v.kind != JAPL_TAG) {
        fprintf(stderr, "japl: get_field on non-tagged value\n");
        abort();
    }
    if (index < 0 || index >= v.tagged_val->field_count) {
        fprintf(stderr, "japl: field index %d out of bounds (count=%d)\n",
                index, v.tagged_val->field_count);
        abort();
    }
    return v.tagged_val->fields[index];
}

/* ── Closures ───────────────────────────────────────────────── */

JaplValue japl_closure(JaplFnPtr fn, int arity, int envc, ...) {
    JaplClosure* c = malloc(sizeof(JaplClosure));
    if (!c) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    c->fn = fn;
    c->arity = arity;
    c->env_count = envc;

    if (envc > 0) {
        c->env = malloc(sizeof(JaplValue) * (size_t)envc);
        if (!c->env) { fprintf(stderr, "japl: out of memory\n"); abort(); }
        va_list ap;
        va_start(ap, envc);
        for (int i = 0; i < envc; i++) {
            c->env[i] = va_arg(ap, JaplValue);
        }
        va_end(ap);
    } else {
        c->env = NULL;
    }

    JaplValue val;
    val.kind = JAPL_FN;
    val.closure_val = c;
    return val;
}

JaplValue japl_closure_v(JaplFnPtr fn, int arity, int envc, JaplValue* env) {
    JaplClosure* c = malloc(sizeof(JaplClosure));
    if (!c) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    c->fn = fn;
    c->arity = arity;
    c->env_count = envc;

    if (envc > 0 && env) {
        c->env = malloc(sizeof(JaplValue) * (size_t)envc);
        if (!c->env) { fprintf(stderr, "japl: out of memory\n"); abort(); }
        memcpy(c->env, env, sizeof(JaplValue) * (size_t)envc);
    } else {
        c->env = NULL;
        c->env_count = 0;
    }

    JaplValue val;
    val.kind = JAPL_FN;
    val.closure_val = c;
    return val;
}

JaplValue japl_apply(JaplValue closure, int argc, ...) {
    if (closure.kind != JAPL_FN) {
        fprintf(stderr, "japl: apply on non-function value\n");
        abort();
    }
    JaplClosure* c = closure.closure_val;

    JaplValue* args = NULL;
    if (argc > 0) {
        args = malloc(sizeof(JaplValue) * (size_t)argc);
        if (!args) { fprintf(stderr, "japl: out of memory\n"); abort(); }
        va_list ap;
        va_start(ap, argc);
        for (int i = 0; i < argc; i++) {
            args[i] = va_arg(ap, JaplValue);
        }
        va_end(ap);
    }

    JaplValue result = c->fn(args, argc, c->env, c->env_count);
    free(args);
    return result;
}

JaplValue japl_apply_v(JaplValue closure, int argc, JaplValue* args) {
    if (closure.kind != JAPL_FN) {
        fprintf(stderr, "japl: apply on non-function value\n");
        abort();
    }
    JaplClosure* c = closure.closure_val;
    return c->fn(args, argc, c->env, c->env_count);
}

/* ── Records ────────────────────────────────────────────────── */

JaplValue japl_record(int count, ...) {
    JaplRecord* r = malloc(sizeof(JaplRecord));
    if (!r) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    r->field_count = count;
    r->keys = malloc(sizeof(const char*) * (size_t)count);
    r->values = malloc(sizeof(JaplValue) * (size_t)count);
    if (!r->keys || !r->values) { fprintf(stderr, "japl: out of memory\n"); abort(); }

    va_list ap;
    va_start(ap, count);
    for (int i = 0; i < count; i++) {
        r->keys[i] = va_arg(ap, const char*);
        r->values[i] = va_arg(ap, JaplValue);
    }
    va_end(ap);

    JaplValue val;
    val.kind = JAPL_RECORD;
    val.record_val = r;
    return val;
}

JaplValue japl_field(JaplValue rec, const char* key) {
    if (rec.kind != JAPL_RECORD) {
        fprintf(stderr, "japl: field access on non-record value\n");
        abort();
    }
    JaplRecord* r = rec.record_val;
    for (int i = 0; i < r->field_count; i++) {
        if (strcmp(r->keys[i], key) == 0) {
            return r->values[i];
        }
    }
    fprintf(stderr, "japl: field '%s' not found in record\n", key);
    abort();
}

JaplValue japl_record_update(JaplValue rec, const char* key, JaplValue val) {
    if (rec.kind != JAPL_RECORD) {
        fprintf(stderr, "japl: record update on non-record value\n");
        abort();
    }
    JaplRecord* old = rec.record_val;
    JaplRecord* r = malloc(sizeof(JaplRecord));
    if (!r) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    r->field_count = old->field_count;
    r->keys = malloc(sizeof(const char*) * (size_t)r->field_count);
    r->values = malloc(sizeof(JaplValue) * (size_t)r->field_count);
    if (!r->keys || !r->values) { fprintf(stderr, "japl: out of memory\n"); abort(); }

    for (int i = 0; i < r->field_count; i++) {
        r->keys[i] = old->keys[i];
        if (strcmp(old->keys[i], key) == 0) {
            r->values[i] = val;
        } else {
            r->values[i] = old->values[i];
        }
    }

    JaplValue result;
    result.kind = JAPL_RECORD;
    result.record_val = r;
    return result;
}

/* ── Extractors ─────────────────────────────────────────────── */

int64_t japl_to_int(JaplValue v) {
    if (v.kind != JAPL_INT) {
        fprintf(stderr, "japl: expected Int, got kind %d\n", v.kind);
        abort();
    }
    return v.int_val;
}

double japl_to_float(JaplValue v) {
    if (v.kind == JAPL_FLOAT) return v.float_val;
    if (v.kind == JAPL_INT) return (double)v.int_val;
    fprintf(stderr, "japl: expected Float, got kind %d\n", v.kind);
    abort();
}

const char* japl_to_cstr(JaplValue v) {
    if (v.kind != JAPL_STRING) {
        fprintf(stderr, "japl: expected String, got kind %d\n", v.kind);
        abort();
    }
    return v.string_val->data;
}

int japl_to_bool(JaplValue v) {
    if (v.kind != JAPL_BOOL) {
        fprintf(stderr, "japl: expected Bool, got kind %d\n", v.kind);
        abort();
    }
    return v.bool_val;
}

/* ── Arithmetic ─────────────────────────────────────────────── */

JaplValue japl_add(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_int(a.int_val + b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_float(a.float_val + b.float_val);
    if (a.kind == JAPL_INT && b.kind == JAPL_FLOAT)
        return japl_float((double)a.int_val + b.float_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_INT)
        return japl_float(a.float_val + (double)b.int_val);
    if (a.kind == JAPL_STRING && b.kind == JAPL_STRING)
        return japl_string_concat(a, b);
    fprintf(stderr, "japl: cannot add kinds %d and %d\n", a.kind, b.kind);
    abort();
}

JaplValue japl_sub(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_int(a.int_val - b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_float(a.float_val - b.float_val);
    if (a.kind == JAPL_INT && b.kind == JAPL_FLOAT)
        return japl_float((double)a.int_val - b.float_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_INT)
        return japl_float(a.float_val - (double)b.int_val);
    fprintf(stderr, "japl: cannot subtract kinds %d and %d\n", a.kind, b.kind);
    abort();
}

JaplValue japl_mul(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_int(a.int_val * b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_float(a.float_val * b.float_val);
    if (a.kind == JAPL_INT && b.kind == JAPL_FLOAT)
        return japl_float((double)a.int_val * b.float_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_INT)
        return japl_float(a.float_val * (double)b.int_val);
    fprintf(stderr, "japl: cannot multiply kinds %d and %d\n", a.kind, b.kind);
    abort();
}

JaplValue japl_div(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT) {
        if (b.int_val == 0) { fprintf(stderr, "japl: division by zero\n"); abort(); }
        return japl_int(a.int_val / b.int_val);
    }
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_float(a.float_val / b.float_val);
    if (a.kind == JAPL_INT && b.kind == JAPL_FLOAT)
        return japl_float((double)a.int_val / b.float_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_INT)
        return japl_float(a.float_val / (double)b.int_val);
    fprintf(stderr, "japl: cannot divide kinds %d and %d\n", a.kind, b.kind);
    abort();
}

JaplValue japl_mod(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT) {
        if (b.int_val == 0) { fprintf(stderr, "japl: modulo by zero\n"); abort(); }
        return japl_int(a.int_val % b.int_val);
    }
    fprintf(stderr, "japl: modulo only supported for Int\n");
    abort();
}

JaplValue japl_negate(JaplValue v) {
    if (v.kind == JAPL_INT) return japl_int(-v.int_val);
    if (v.kind == JAPL_FLOAT) return japl_float(-v.float_val);
    fprintf(stderr, "japl: cannot negate kind %d\n", v.kind);
    abort();
}

/* ── Comparison ─────────────────────────────────────────────── */

static int japl_values_equal(JaplValue a, JaplValue b) {
    if (a.kind != b.kind) return 0;
    switch (a.kind) {
        case JAPL_INT:    return a.int_val == b.int_val;
        case JAPL_FLOAT:  return a.float_val == b.float_val;
        case JAPL_BOOL:   return a.bool_val == b.bool_val;
        case JAPL_STRING: return strcmp(a.string_val->data, b.string_val->data) == 0;
        case JAPL_UNIT:   return 1;
        case JAPL_NIL:    return 1;
        case JAPL_PID:    return a.pid_val == b.pid_val;
        case JAPL_TAG:
            if (strcmp(a.tagged_val->tag, b.tagged_val->tag) != 0) return 0;
            if (a.tagged_val->field_count != b.tagged_val->field_count) return 0;
            for (int i = 0; i < a.tagged_val->field_count; i++) {
                if (!japl_values_equal(a.tagged_val->fields[i], b.tagged_val->fields[i]))
                    return 0;
            }
            return 1;
        default: return 0;
    }
}

JaplValue japl_eq(JaplValue a, JaplValue b) {
    return japl_bool(japl_values_equal(a, b));
}

JaplValue japl_neq(JaplValue a, JaplValue b) {
    return japl_bool(!japl_values_equal(a, b));
}

JaplValue japl_lt(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_bool(a.int_val < b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_bool(a.float_val < b.float_val);
    if (a.kind == JAPL_STRING && b.kind == JAPL_STRING)
        return japl_bool(strcmp(a.string_val->data, b.string_val->data) < 0);
    fprintf(stderr, "japl: cannot compare kinds %d and %d with <\n", a.kind, b.kind);
    abort();
}

JaplValue japl_gt(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_bool(a.int_val > b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_bool(a.float_val > b.float_val);
    if (a.kind == JAPL_STRING && b.kind == JAPL_STRING)
        return japl_bool(strcmp(a.string_val->data, b.string_val->data) > 0);
    fprintf(stderr, "japl: cannot compare kinds %d and %d with >\n", a.kind, b.kind);
    abort();
}

JaplValue japl_lte(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_bool(a.int_val <= b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_bool(a.float_val <= b.float_val);
    fprintf(stderr, "japl: cannot compare kinds %d and %d with <=\n", a.kind, b.kind);
    abort();
}

JaplValue japl_gte(JaplValue a, JaplValue b) {
    if (a.kind == JAPL_INT && b.kind == JAPL_INT)
        return japl_bool(a.int_val >= b.int_val);
    if (a.kind == JAPL_FLOAT && b.kind == JAPL_FLOAT)
        return japl_bool(a.float_val >= b.float_val);
    fprintf(stderr, "japl: cannot compare kinds %d and %d with >=\n", a.kind, b.kind);
    abort();
}

JaplValue japl_not(JaplValue v) {
    if (v.kind != JAPL_BOOL) {
        fprintf(stderr, "japl: not on non-boolean\n");
        abort();
    }
    return japl_bool(!v.bool_val);
}

/* ── Display ────────────────────────────────────────────────── */

char* japl_show(JaplValue v) {
    char* buf = NULL;
    int len = 0;

    switch (v.kind) {
        case JAPL_INT:
            len = snprintf(NULL, 0, "%" PRId64, v.int_val);
            buf = malloc((size_t)len + 1);
            snprintf(buf, (size_t)len + 1, "%" PRId64, v.int_val);
            break;
        case JAPL_FLOAT:
            len = snprintf(NULL, 0, "%g", v.float_val);
            buf = malloc((size_t)len + 1);
            snprintf(buf, (size_t)len + 1, "%g", v.float_val);
            break;
        case JAPL_BOOL:
            buf = strdup(v.bool_val ? "true" : "false");
            break;
        case JAPL_STRING:
            buf = strdup(v.string_val->data);
            break;
        case JAPL_UNIT:
            buf = strdup("()");
            break;
        case JAPL_NIL:
            buf = strdup("[]");
            break;
        case JAPL_PID:
            len = snprintf(NULL, 0, "<pid:%" PRIu64 ">", v.pid_val);
            buf = malloc((size_t)len + 1);
            snprintf(buf, (size_t)len + 1, "<pid:%" PRIu64 ">", v.pid_val);
            break;
        case JAPL_TAG: {
            /* Format: Tag(field1, field2, ...) or just Tag */
            if (v.tagged_val->field_count == 0) {
                buf = strdup(v.tagged_val->tag);
            } else {
                /* Build up the string */
                size_t total = strlen(v.tagged_val->tag) + 2; /* Tag( ... ) */
                char** parts = malloc(sizeof(char*) * (size_t)v.tagged_val->field_count);
                for (int i = 0; i < v.tagged_val->field_count; i++) {
                    parts[i] = japl_show(v.tagged_val->fields[i]);
                    total += strlen(parts[i]) + 2; /* ", " */
                }
                buf = malloc(total + 1);
                strcpy(buf, v.tagged_val->tag);
                strcat(buf, "(");
                for (int i = 0; i < v.tagged_val->field_count; i++) {
                    if (i > 0) strcat(buf, ", ");
                    strcat(buf, parts[i]);
                    free(parts[i]);
                }
                strcat(buf, ")");
                free(parts);
            }
            break;
        }
        case JAPL_FN:
            buf = strdup("<fn>");
            break;
        case JAPL_LIST: {
            /* Format: [1, 2, 3] */
            size_t total = 3; /* "[]" + null */
            int count = 0;
            char** parts = NULL;

            /* Walk the list to get all elements */
            JaplValue cur = v;
            while (cur.kind == JAPL_LIST && cur.list_val != NULL) {
                count++;
                parts = realloc(parts, sizeof(char*) * (size_t)count);
                parts[count - 1] = japl_show(cur.list_val->head);
                total += strlen(parts[count - 1]) + 2;
                if (cur.list_val->tail) {
                    JaplValue next;
                    next.kind = JAPL_LIST;
                    next.list_val = cur.list_val->tail;
                    cur = next;
                } else {
                    break;
                }
            }

            buf = malloc(total + 1);
            strcpy(buf, "[");
            for (int i = 0; i < count; i++) {
                if (i > 0) strcat(buf, ", ");
                strcat(buf, parts[i]);
                free(parts[i]);
            }
            strcat(buf, "]");
            free(parts);
            break;
        }
        case JAPL_RECORD: {
            /* Format: { key1: val1, key2: val2 } */
            JaplRecord* r = v.record_val;
            size_t total = 5;
            char** val_parts = malloc(sizeof(char*) * (size_t)r->field_count);
            for (int i = 0; i < r->field_count; i++) {
                val_parts[i] = japl_show(r->values[i]);
                total += strlen(r->keys[i]) + strlen(val_parts[i]) + 4;
            }
            buf = malloc(total + 1);
            strcpy(buf, "{ ");
            for (int i = 0; i < r->field_count; i++) {
                if (i > 0) strcat(buf, ", ");
                strcat(buf, r->keys[i]);
                strcat(buf, ": ");
                strcat(buf, val_parts[i]);
                free(val_parts[i]);
            }
            strcat(buf, " }");
            free(val_parts);
            break;
        }
    }
    return buf;
}

void japl_print(JaplValue v) {
    char* s = japl_show(v);
    fputs(s, stdout);
    free(s);
}

void japl_println(JaplValue v) {
    char* s = japl_show(v);
    puts(s);
    free(s);
}
