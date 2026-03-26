#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "japl_supervisor.h"

JaplSupervisor* japl_supervisor_new(SuperStrategy strategy, int max_restarts, int max_seconds) {
    JaplSupervisor* sup = calloc(1, sizeof(JaplSupervisor));
    if (!sup) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    sup->strategy = strategy;
    sup->max_restarts = max_restarts;
    sup->max_seconds = max_seconds;
    sup->child_count = 0;
    sup->restart_count = 0;
    sup->window_start = time(NULL);
    return sup;
}

void japl_supervisor_add_child(JaplSupervisor* sup, const char* id,
                                JaplFnPtr start_fn, JaplValue start_arg,
                                RestartPolicy restart) {
    if (sup->child_count >= JAPL_SUPERVISOR_MAX_CHILDREN) {
        fprintf(stderr, "japl: supervisor max children reached\n");
        return;
    }
    int idx = sup->child_count++;
    sup->children[idx].id = id;
    sup->children[idx].start_fn = start_fn;
    sup->children[idx].start_arg = start_arg;
    sup->children[idx].restart = restart;
    sup->children[idx].pid = 0;
}

static void start_child(ChildSpec* child) {
    child->pid = japl_spawn(child->start_fn, child->start_arg);
}

uint64_t japl_supervisor_start(JaplSupervisor* spec) {
    spec->window_start = time(NULL);
    spec->restart_count = 0;

    for (int i = 0; i < spec->child_count; i++) {
        start_child(&spec->children[i]);
    }

    /* Return the PID of the first child as a reference, or 0 if no children */
    return spec->child_count > 0 ? spec->children[0].pid : 0;
}

int japl_supervisor_restart_child(JaplSupervisor* sup, int index) {
    if (index < 0 || index >= sup->child_count) return -1;

    /* Check restart limits */
    time_t now = time(NULL);
    if (difftime(now, sup->window_start) > sup->max_seconds) {
        sup->restart_count = 0;
        sup->window_start = now;
    }

    sup->restart_count++;
    if (sup->restart_count > sup->max_restarts) {
        fprintf(stderr, "japl: supervisor max restarts exceeded\n");
        return -1;
    }

    switch (sup->strategy) {
        case ONE_FOR_ONE:
            start_child(&sup->children[index]);
            break;
        case ALL_FOR_ONE:
            for (int i = 0; i < sup->child_count; i++) {
                start_child(&sup->children[i]);
            }
            break;
        case REST_FOR_ONE:
            for (int i = index; i < sup->child_count; i++) {
                start_child(&sup->children[i]);
            }
            break;
    }

    return 0;
}

void japl_supervisor_free(JaplSupervisor* sup) {
    free(sup);
}
