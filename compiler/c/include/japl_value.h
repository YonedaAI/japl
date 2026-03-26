#ifndef JAPL_VALUE_H
#define JAPL_VALUE_H

#include <stdint.h>
#include <stddef.h>
#include <stdarg.h>

/* Forward declarations */
struct JaplString;
struct JaplList;
struct JaplRecord;
struct JaplTagged;
struct JaplClosure;

/* ── Value kinds ────────────────────────────────────────────── */

typedef enum {
    JAPL_INT,
    JAPL_FLOAT,
    JAPL_BOOL,
    JAPL_STRING,
    JAPL_LIST,
    JAPL_RECORD,
    JAPL_TAG,
    JAPL_FN,
    JAPL_PID,
    JAPL_UNIT,
    JAPL_NIL,
} JaplValueKind;

/* ── Core value type ────────────────────────────────────────── */

typedef struct JaplValue {
    JaplValueKind kind;
    union {
        int64_t int_val;
        double float_val;
        int bool_val;
        struct JaplString* string_val;
        struct JaplList* list_val;
        struct JaplRecord* record_val;
        struct JaplTagged* tagged_val;
        struct JaplClosure* closure_val;
        uint64_t pid_val;
    };
} JaplValue;

/* ── Tagged unions ──────────────────────────────────────────── */

#define JAPL_TAG_MAX_FIELDS 8

typedef struct JaplTagged {
    const char* tag;
    int field_count;
    JaplValue fields[JAPL_TAG_MAX_FIELDS];
} JaplTagged;

/* ── Closures ───────────────────────────────────────────────── */

typedef JaplValue (*JaplFnPtr)(JaplValue* args, int argc, JaplValue* env, int envc);

typedef struct JaplClosure {
    JaplFnPtr fn;
    JaplValue* env;
    int env_count;
    int arity;
} JaplClosure;

/* ── Records ────────────────────────────────────────────────── */

typedef struct JaplRecord {
    int field_count;
    const char** keys;
    JaplValue* values;
} JaplRecord;

/* ── Constructors ───────────────────────────────────────────── */

JaplValue japl_int(int64_t v);
JaplValue japl_float(double v);
JaplValue japl_bool(int v);
JaplValue japl_unit(void);
JaplValue japl_nil(void);
JaplValue japl_pid(uint64_t pid);

/* ── Tagged union constructors ──────────────────────────────── */

JaplValue japl_tagged(const char* tag, int count, ...);
JaplValue japl_tagged_v(const char* tag, int count, JaplValue* fields);
const char* japl_get_tag(JaplValue v);
JaplValue japl_get_field(JaplValue v, int index);

/* ── Closure constructors ───────────────────────────────────── */

JaplValue japl_closure(JaplFnPtr fn, int arity, int envc, ...);
JaplValue japl_closure_v(JaplFnPtr fn, int arity, int envc, JaplValue* env);
JaplValue japl_apply(JaplValue closure, int argc, ...);
JaplValue japl_apply_v(JaplValue closure, int argc, JaplValue* args);

/* ── Record constructors ────────────────────────────────────── */

JaplValue japl_record(int count, ...);
JaplValue japl_field(JaplValue rec, const char* key);
JaplValue japl_record_update(JaplValue rec, const char* key, JaplValue val);

/* ── Extractors ─────────────────────────────────────────────── */

int64_t japl_to_int(JaplValue v);
double japl_to_float(JaplValue v);
const char* japl_to_cstr(JaplValue v);
int japl_to_bool(JaplValue v);

/* ── Arithmetic ─────────────────────────────────────────────── */

JaplValue japl_add(JaplValue a, JaplValue b);
JaplValue japl_sub(JaplValue a, JaplValue b);
JaplValue japl_mul(JaplValue a, JaplValue b);
JaplValue japl_div(JaplValue a, JaplValue b);
JaplValue japl_mod(JaplValue a, JaplValue b);
JaplValue japl_negate(JaplValue v);

/* ── Comparison ─────────────────────────────────────────────── */

JaplValue japl_eq(JaplValue a, JaplValue b);
JaplValue japl_neq(JaplValue a, JaplValue b);
JaplValue japl_lt(JaplValue a, JaplValue b);
JaplValue japl_gt(JaplValue a, JaplValue b);
JaplValue japl_lte(JaplValue a, JaplValue b);
JaplValue japl_gte(JaplValue a, JaplValue b);
JaplValue japl_not(JaplValue v);

/* ── Display ────────────────────────────────────────────────── */

void japl_print(JaplValue v);
void japl_println(JaplValue v);
char* japl_show(JaplValue v);

#endif /* JAPL_VALUE_H */
