#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include "japl_scheduler.h"

/* ── Global scheduler state ─────────────────────────────────── */

static JaplProcess* processes[JAPL_MAX_PROCESSES];
static pthread_mutex_t sched_lock = PTHREAD_MUTEX_INITIALIZER;
static uint64_t next_pid = 1;
static int scheduler_initialized = 0;

/* Thread-local current PID */
static pthread_key_t current_pid_key;
static int key_created = 0;

/* ── Internal helpers ───────────────────────────────────────── */

static void* process_thread_entry(void* arg) {
    JaplProcess* proc = (JaplProcess*)arg;

    /* Set thread-local PID */
    japl_set_current_pid(proc->pid);

    /* Run the entry function */
    JaplValue args[1];
    args[0] = proc->arg;
    proc->state = PROC_RUNNING;
    proc->result = proc->entry(args, 1, NULL, 0);
    proc->state = PROC_DONE;

    return NULL;
}

/* ── Public API ─────────────────────────────────────────────── */

void japl_scheduler_init(void) {
    if (scheduler_initialized) return;

    if (!key_created) {
        pthread_key_create(&current_pid_key, NULL);
        key_created = 1;
    }

    memset(processes, 0, sizeof(processes));
    next_pid = 1;
    scheduler_initialized = 1;
}

void japl_scheduler_shutdown(void) {
    if (!scheduler_initialized) return;

    pthread_mutex_lock(&sched_lock);
    for (int i = 0; i < JAPL_MAX_PROCESSES; i++) {
        if (processes[i]) {
            if (processes[i]->thread_started) {
                pthread_join(processes[i]->thread, NULL);
            }
            if (processes[i]->mailbox) {
                japl_mailbox_free(processes[i]->mailbox);
            }
            free(processes[i]);
            processes[i] = NULL;
        }
    }
    scheduler_initialized = 0;
    next_pid = 1;
    pthread_mutex_unlock(&sched_lock);
}

uint64_t japl_spawn(JaplFnPtr fn, JaplValue arg) {
    if (!scheduler_initialized) {
        japl_scheduler_init();
    }

    pthread_mutex_lock(&sched_lock);

    uint64_t pid = next_pid++;
    int slot = (int)(pid % JAPL_MAX_PROCESSES);

    /* Clean up old process in this slot if needed */
    if (processes[slot]) {
        if (processes[slot]->thread_started) {
            pthread_join(processes[slot]->thread, NULL);
        }
        if (processes[slot]->mailbox) {
            japl_mailbox_free(processes[slot]->mailbox);
        }
        free(processes[slot]);
    }

    JaplProcess* proc = calloc(1, sizeof(JaplProcess));
    if (!proc) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    proc->pid = pid;
    proc->state = PROC_READY;
    proc->entry = fn;
    proc->arg = arg;
    proc->parent = japl_self();
    proc->mailbox = japl_mailbox_new(JAPL_MAILBOX_DEFAULT_CAPACITY);
    proc->thread_started = 0;

    processes[slot] = proc;

    /* Start a real pthread for this process */
    proc->thread_started = 1;
    int err = pthread_create(&proc->thread, NULL, process_thread_entry, proc);
    if (err != 0) {
        fprintf(stderr, "japl: failed to create thread: %d\n", err);
        proc->state = PROC_FAILED;
        proc->thread_started = 0;
    }

    pthread_mutex_unlock(&sched_lock);
    return pid;
}

void japl_send(uint64_t pid, JaplValue msg) {
    JaplProcess* proc = japl_process_lookup(pid);
    if (proc && proc->mailbox) {
        japl_mailbox_send(proc->mailbox, msg);
    }
}

JaplValue japl_receive(void) {
    uint64_t pid = japl_self();
    JaplProcess* proc = japl_process_lookup(pid);
    if (!proc || !proc->mailbox) {
        fprintf(stderr, "japl: receive called with no current process\n");
        abort();
    }
    return japl_mailbox_receive(proc->mailbox);
}

uint64_t japl_self(void) {
    if (!key_created) return 0;
    void* val = pthread_getspecific(current_pid_key);
    return (uint64_t)(uintptr_t)val;
}

void japl_set_current_pid(uint64_t pid) {
    if (!key_created) {
        pthread_key_create(&current_pid_key, NULL);
        key_created = 1;
    }
    pthread_setspecific(current_pid_key, (void*)(uintptr_t)pid);
}

void japl_yield(void) {
    sched_yield();
}

JaplProcess* japl_process_lookup(uint64_t pid) {
    if (pid == 0) return NULL;
    int slot = (int)(pid % JAPL_MAX_PROCESSES);
    pthread_mutex_lock(&sched_lock);
    JaplProcess* proc = processes[slot];
    if (proc && proc->pid == pid) {
        pthread_mutex_unlock(&sched_lock);
        return proc;
    }
    pthread_mutex_unlock(&sched_lock);
    return NULL;
}
