#include <assert.h>
#include <string.h>
#include <unistd.h>
#include "japl_runtime.h"

static JaplValue worker_fn(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)argc; (void)env; (void)envc;
    return args[0];
}

void test_supervisor_create(void) {
    JaplSupervisor* sup = japl_supervisor_new(ONE_FOR_ONE, 5, 60);
    assert(sup != NULL);
    assert(sup->strategy == ONE_FOR_ONE);
    assert(sup->max_restarts == 5);
    assert(sup->child_count == 0);
    japl_supervisor_free(sup);
}

void test_supervisor_add_child(void) {
    JaplSupervisor* sup = japl_supervisor_new(ONE_FOR_ONE, 5, 60);
    japl_supervisor_add_child(sup, "worker1", worker_fn, japl_int(1), PERMANENT);
    assert(sup->child_count == 1);
    assert(strcmp(sup->children[0].id, "worker1") == 0);
    japl_supervisor_free(sup);
}

void test_supervisor_start(void) {
    japl_scheduler_init();
    JaplSupervisor* sup = japl_supervisor_new(ONE_FOR_ONE, 5, 60);
    japl_supervisor_add_child(sup, "w1", worker_fn, japl_int(1), PERMANENT);
    japl_supervisor_add_child(sup, "w2", worker_fn, japl_int(2), TRANSIENT);

    uint64_t pid = japl_supervisor_start(sup);
    assert(pid > 0);
    assert(sup->children[0].pid > 0);
    assert(sup->children[1].pid > 0);

    usleep(50000); /* Let workers finish */
    japl_supervisor_free(sup);
    japl_scheduler_shutdown();
}

void test_supervisor_restart(void) {
    japl_scheduler_init();
    JaplSupervisor* sup = japl_supervisor_new(ONE_FOR_ONE, 5, 60);
    japl_supervisor_add_child(sup, "w1", worker_fn, japl_int(1), PERMANENT);
    japl_supervisor_start(sup);
    usleep(50000);

    uint64_t old_pid = sup->children[0].pid;
    int result = japl_supervisor_restart_child(sup, 0);
    assert(result == 0);
    assert(sup->children[0].pid != old_pid);

    usleep(50000);
    japl_supervisor_free(sup);
    japl_scheduler_shutdown();
}
