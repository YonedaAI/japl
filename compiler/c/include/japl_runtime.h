#ifndef JAPL_RUNTIME_H
#define JAPL_RUNTIME_H

/*
 * JAPL Runtime — Master Header
 *
 * Include this single header to get the full JAPL C runtime.
 */

#include "japl_value.h"
#include "japl_string.h"
#include "japl_list.h"
#include "japl_map.h"
#include "japl_gc.h"
#include "japl_mailbox.h"
#include "japl_scheduler.h"
#include "japl_supervisor.h"

/* ── Standard library builtins ──────────────────────────────── */

JaplValue japl_builtin_println(JaplValue* args, int argc, JaplValue* env, int envc);
JaplValue japl_builtin_print(JaplValue* args, int argc, JaplValue* env, int envc);
JaplValue japl_builtin_show(JaplValue* args, int argc, JaplValue* env, int envc);
JaplValue japl_builtin_int_to_string(JaplValue* args, int argc, JaplValue* env, int envc);
JaplValue japl_builtin_string_length(JaplValue* args, int argc, JaplValue* env, int envc);

/* ── Runtime init/shutdown ──────────────────────────────────── */

void japl_runtime_init(void);
void japl_runtime_shutdown(void);

#endif /* JAPL_RUNTIME_H */
