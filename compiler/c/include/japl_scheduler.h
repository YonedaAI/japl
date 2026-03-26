#ifndef JAPL_SCHEDULER_H
#define JAPL_SCHEDULER_H

#include <stdint.h>
#include <pthread.h>
#include "japl_value.h"
#include "japl_mailbox.h"

#define JAPL_MAX_PROCESSES 65536
#define JAPL_STACK_SIZE (64 * 1024)  /* 64 KB per process */

typedef enum {
    PROC_READY,
    PROC_RUNNING,
    PROC_WAITING,
    PROC_DONE,
    PROC_FAILED,
} ProcState;

typedef struct JaplProcess {
    uint64_t pid;
    ProcState state;
    JaplMailbox* mailbox;
    uint64_t parent;
    JaplFnPtr entry;
    JaplValue arg;
    JaplValue result;
    pthread_t thread;
    int thread_started;
} JaplProcess;

/* Initialize the scheduler. */
void japl_scheduler_init(void);

/* Shut down and free all scheduler resources. */
void japl_scheduler_shutdown(void);

/* Spawn a new process that runs fn(arg). Returns PID. */
uint64_t japl_spawn(JaplFnPtr fn, JaplValue arg);

/* Send a message to a process by PID. */
void japl_send(uint64_t pid, JaplValue msg);

/* Receive a message for the current process (blocks). */
JaplValue japl_receive(void);

/* Get the current process PID. */
uint64_t japl_self(void);

/* Yield the current process (no-op in pthread model). */
void japl_yield(void);

/* Look up a process by PID (internal). */
JaplProcess* japl_process_lookup(uint64_t pid);

/* Set the "current" PID for the calling OS thread. */
void japl_set_current_pid(uint64_t pid);

#endif /* JAPL_SCHEDULER_H */
