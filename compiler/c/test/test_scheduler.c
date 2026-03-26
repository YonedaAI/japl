#include <assert.h>
#include <unistd.h>
#include "japl_runtime.h"

/* Simple process function that returns its argument + 1 */
static JaplValue echo_fn(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)env; (void)envc; (void)argc;
    return args[0];
}

void test_scheduler_init(void) {
    japl_scheduler_init();
    japl_scheduler_shutdown();
    /* Should not crash when called multiple times */
    japl_scheduler_init();
    japl_scheduler_shutdown();
}

void test_scheduler_spawn(void) {
    japl_scheduler_init();
    uint64_t pid = japl_spawn(echo_fn, japl_int(42));
    assert(pid > 0);
    usleep(50000); /* Let thread finish */
    japl_scheduler_shutdown();
}

void test_scheduler_spawn_multiple(void) {
    japl_scheduler_init();
    uint64_t p1 = japl_spawn(echo_fn, japl_int(1));
    uint64_t p2 = japl_spawn(echo_fn, japl_int(2));
    uint64_t p3 = japl_spawn(echo_fn, japl_int(3));
    assert(p1 != p2);
    assert(p2 != p3);
    usleep(50000);
    japl_scheduler_shutdown();
}

/* Process that receives a message and stores it */
static JaplValue receiver_fn(JaplValue* args, int argc, JaplValue* env, int envc) {
    (void)args; (void)argc; (void)env; (void)envc;
    JaplValue msg = japl_receive();
    return msg;
}

void test_scheduler_send_receive(void) {
    japl_scheduler_init();
    uint64_t pid = japl_spawn(receiver_fn, japl_unit());
    usleep(10000); /* Let process start and block on receive */
    japl_send(pid, japl_int(99));
    usleep(50000); /* Let process finish */

    JaplProcess* proc = japl_process_lookup(pid);
    assert(proc != NULL);
    assert(proc->state == PROC_DONE);
    assert(japl_to_int(proc->result) == 99);

    japl_scheduler_shutdown();
}

void test_scheduler_process_lookup(void) {
    japl_scheduler_init();
    uint64_t pid = japl_spawn(echo_fn, japl_int(1));
    JaplProcess* proc = japl_process_lookup(pid);
    assert(proc != NULL);
    assert(proc->pid == pid);
    assert(japl_process_lookup(999999) == NULL);
    usleep(50000);
    japl_scheduler_shutdown();
}
