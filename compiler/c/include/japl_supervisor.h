#ifndef JAPL_SUPERVISOR_H
#define JAPL_SUPERVISOR_H

#include <time.h>
#include "japl_value.h"
#include "japl_scheduler.h"

typedef enum {
    ONE_FOR_ONE,
    ALL_FOR_ONE,
    REST_FOR_ONE,
} SuperStrategy;

typedef enum {
    PERMANENT,
    TRANSIENT,
    TEMPORARY,
} RestartPolicy;

#define JAPL_SUPERVISOR_MAX_CHILDREN 64

typedef struct {
    const char* id;
    JaplFnPtr start_fn;
    JaplValue start_arg;
    RestartPolicy restart;
    uint64_t pid;
} ChildSpec;

typedef struct {
    SuperStrategy strategy;
    int max_restarts;
    int max_seconds;
    ChildSpec children[JAPL_SUPERVISOR_MAX_CHILDREN];
    int child_count;
    int restart_count;
    time_t window_start;
} JaplSupervisor;

/* Create a new supervisor spec. */
JaplSupervisor* japl_supervisor_new(SuperStrategy strategy, int max_restarts, int max_seconds);

/* Add a child spec to the supervisor. */
void japl_supervisor_add_child(JaplSupervisor* sup, const char* id,
                                JaplFnPtr start_fn, JaplValue start_arg,
                                RestartPolicy restart);

/* Start the supervisor and all its children. Returns supervisor PID. */
uint64_t japl_supervisor_start(JaplSupervisor* spec);

/* Restart a child by index. */
int japl_supervisor_restart_child(JaplSupervisor* sup, int index);

/* Free supervisor resources. */
void japl_supervisor_free(JaplSupervisor* sup);

#endif /* JAPL_SUPERVISOR_H */
