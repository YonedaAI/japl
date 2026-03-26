#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
#include "japl_runtime.h"

JaplValue japl_builtin_println(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc;
    if (argc < 1) return japl_unit();
    japl_println(args[0]);
    return japl_unit();
}

JaplValue japl_builtin_print(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc;
    if (argc < 1) return japl_unit();
    japl_print(args[0]);
    return japl_unit();
}

JaplValue japl_builtin_show(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc;
    if (argc < 1) return japl_string("");
    char* s = japl_show(args[0]);
    JaplValue result = japl_string(s);
    free(s);
    return result;
}

JaplValue japl_builtin_int_to_string(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc;
    if (argc < 1) return japl_string("");
    int64_t n = japl_to_int(args[0]);
    char buf[32];
    snprintf(buf, sizeof(buf), "%" PRId64, n);
    return japl_string(buf);
}

JaplValue japl_builtin_string_length(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc;
    if (argc < 1) return japl_int(0);
    return japl_int(japl_string_length(args[0]));
}

/* ── Runtime init/shutdown ──────────────────────────────────── */

void japl_runtime_init(void) {
    japl_scheduler_init();
}

void japl_runtime_shutdown(void) {
    japl_scheduler_shutdown();
}
