#include <stdio.h>
#include <assert.h>

#define TEST(name) do { \
    printf("  %-50s", #name); \
    fflush(stdout); \
    name(); \
    printf("OK\n"); \
} while(0)

/* ── Value tests ────────────────────────────────────────────── */
extern void test_value_int(void);
extern void test_value_int_negative(void);
extern void test_value_int_zero(void);
extern void test_value_float(void);
extern void test_value_float_from_int(void);
extern void test_value_bool_true(void);
extern void test_value_bool_false(void);
extern void test_value_string(void);
extern void test_value_string_empty(void);
extern void test_value_string_concat(void);
extern void test_value_unit(void);
extern void test_value_nil(void);
extern void test_value_add_int(void);
extern void test_value_add_float(void);
extern void test_value_add_mixed(void);
extern void test_value_sub(void);
extern void test_value_mul(void);
extern void test_value_div(void);
extern void test_value_mod(void);
extern void test_value_negate(void);
extern void test_value_eq_int(void);
extern void test_value_eq_string(void);
extern void test_value_neq(void);
extern void test_value_lt(void);
extern void test_value_gt(void);
extern void test_value_not(void);
extern void test_value_show_int(void);
extern void test_value_show_bool(void);
extern void test_value_show_unit(void);
extern void test_value_show_nil(void);
extern void test_tagged_none(void);
extern void test_tagged_some(void);
extern void test_tagged_show(void);
extern void test_tagged_eq(void);
extern void test_closure_basic(void);
extern void test_closure_with_env(void);
extern void test_record_basic(void);
extern void test_record_update(void);

/* ── GC tests ───────────────────────────────────────────────── */
extern void test_gc_heap_create(void);
extern void test_gc_alloc(void);
extern void test_gc_multiple_allocs(void);
extern void test_gc_free_all(void);
extern void test_gc_collect_noop(void);
extern void test_gc_write_to_alloc(void);

/* ── Scheduler tests ────────────────────────────────────────── */
extern void test_scheduler_init(void);
extern void test_scheduler_spawn(void);
extern void test_scheduler_spawn_multiple(void);
extern void test_scheduler_send_receive(void);
extern void test_scheduler_process_lookup(void);

/* ── Mailbox tests ──────────────────────────────────────────── */
extern void test_mailbox_create(void);
extern void test_mailbox_send_receive(void);
extern void test_mailbox_fifo(void);
extern void test_mailbox_try_receive(void);
extern void test_mailbox_grow(void);
extern void test_mailbox_concurrent(void);

/* ── Supervisor tests ───────────────────────────────────────── */
extern void test_supervisor_create(void);
extern void test_supervisor_add_child(void);
extern void test_supervisor_start(void);
extern void test_supervisor_restart(void);

int main(void) {
    printf("JAPL Runtime Tests\n");
    printf("==================\n\n");

    printf("[Values]\n");
    TEST(test_value_int);
    TEST(test_value_int_negative);
    TEST(test_value_int_zero);
    TEST(test_value_float);
    TEST(test_value_float_from_int);
    TEST(test_value_bool_true);
    TEST(test_value_bool_false);
    TEST(test_value_string);
    TEST(test_value_string_empty);
    TEST(test_value_string_concat);
    TEST(test_value_unit);
    TEST(test_value_nil);

    printf("\n[Arithmetic]\n");
    TEST(test_value_add_int);
    TEST(test_value_add_float);
    TEST(test_value_add_mixed);
    TEST(test_value_sub);
    TEST(test_value_mul);
    TEST(test_value_div);
    TEST(test_value_mod);
    TEST(test_value_negate);

    printf("\n[Comparison]\n");
    TEST(test_value_eq_int);
    TEST(test_value_eq_string);
    TEST(test_value_neq);
    TEST(test_value_lt);
    TEST(test_value_gt);
    TEST(test_value_not);

    printf("\n[Show]\n");
    TEST(test_value_show_int);
    TEST(test_value_show_bool);
    TEST(test_value_show_unit);
    TEST(test_value_show_nil);

    printf("\n[Tagged Unions]\n");
    TEST(test_tagged_none);
    TEST(test_tagged_some);
    TEST(test_tagged_show);
    TEST(test_tagged_eq);

    printf("\n[Closures]\n");
    TEST(test_closure_basic);
    TEST(test_closure_with_env);

    printf("\n[Records]\n");
    TEST(test_record_basic);
    TEST(test_record_update);

    printf("\n[GC]\n");
    TEST(test_gc_heap_create);
    TEST(test_gc_alloc);
    TEST(test_gc_multiple_allocs);
    TEST(test_gc_free_all);
    TEST(test_gc_collect_noop);
    TEST(test_gc_write_to_alloc);

    printf("\n[Mailbox]\n");
    TEST(test_mailbox_create);
    TEST(test_mailbox_send_receive);
    TEST(test_mailbox_fifo);
    TEST(test_mailbox_try_receive);
    TEST(test_mailbox_grow);
    TEST(test_mailbox_concurrent);

    printf("\n[Scheduler]\n");
    TEST(test_scheduler_init);
    TEST(test_scheduler_spawn);
    TEST(test_scheduler_spawn_multiple);
    TEST(test_scheduler_send_receive);
    TEST(test_scheduler_process_lookup);

    printf("\n[Supervisor]\n");
    TEST(test_supervisor_create);
    TEST(test_supervisor_add_child);
    TEST(test_supervisor_start);
    TEST(test_supervisor_restart);

    printf("\n==================\n");
    printf("All 50 tests passed!\n");
    return 0;
}
