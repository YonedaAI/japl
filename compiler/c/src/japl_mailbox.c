#include <stdio.h>
#include <stdlib.h>
#include "japl_mailbox.h"

JaplMailbox* japl_mailbox_new(int capacity) {
    if (capacity <= 0) capacity = JAPL_MAILBOX_DEFAULT_CAPACITY;

    JaplMailbox* mb = malloc(sizeof(JaplMailbox));
    if (!mb) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    mb->capacity = capacity;
    mb->head = 0;
    mb->tail = 0;
    mb->count = 0;
    mb->buffer = malloc(sizeof(JaplValue) * (size_t)capacity);
    if (!mb->buffer) { fprintf(stderr, "japl: out of memory\n"); abort(); }

    pthread_mutex_init(&mb->lock, NULL);
    pthread_cond_init(&mb->not_empty, NULL);
    return mb;
}

void japl_mailbox_send(JaplMailbox* mb, JaplValue msg) {
    pthread_mutex_lock(&mb->lock);

    /* If full, grow the buffer */
    if (mb->count >= mb->capacity) {
        int new_cap = mb->capacity * 2;
        JaplValue* new_buf = malloc(sizeof(JaplValue) * (size_t)new_cap);
        if (!new_buf) { fprintf(stderr, "japl: out of memory\n"); abort(); }

        /* Copy in order */
        for (int i = 0; i < mb->count; i++) {
            new_buf[i] = mb->buffer[(mb->head + i) % mb->capacity];
        }
        free(mb->buffer);
        mb->buffer = new_buf;
        mb->head = 0;
        mb->tail = mb->count;
        mb->capacity = new_cap;
    }

    mb->buffer[mb->tail] = msg;
    mb->tail = (mb->tail + 1) % mb->capacity;
    mb->count++;

    pthread_cond_signal(&mb->not_empty);
    pthread_mutex_unlock(&mb->lock);
}

JaplValue japl_mailbox_receive(JaplMailbox* mb) {
    pthread_mutex_lock(&mb->lock);

    while (mb->count == 0) {
        pthread_cond_wait(&mb->not_empty, &mb->lock);
    }

    JaplValue msg = mb->buffer[mb->head];
    mb->head = (mb->head + 1) % mb->capacity;
    mb->count--;

    pthread_mutex_unlock(&mb->lock);
    return msg;
}

int japl_mailbox_try_receive(JaplMailbox* mb, JaplValue* out) {
    pthread_mutex_lock(&mb->lock);

    if (mb->count == 0) {
        pthread_mutex_unlock(&mb->lock);
        return 0;
    }

    *out = mb->buffer[mb->head];
    mb->head = (mb->head + 1) % mb->capacity;
    mb->count--;

    pthread_mutex_unlock(&mb->lock);
    return 1;
}

int japl_mailbox_count(JaplMailbox* mb) {
    pthread_mutex_lock(&mb->lock);
    int c = mb->count;
    pthread_mutex_unlock(&mb->lock);
    return c;
}

void japl_mailbox_free(JaplMailbox* mb) {
    if (mb) {
        pthread_mutex_destroy(&mb->lock);
        pthread_cond_destroy(&mb->not_empty);
        free(mb->buffer);
        free(mb);
    }
}
