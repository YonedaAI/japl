#ifndef JAPL_MAILBOX_H
#define JAPL_MAILBOX_H

#include <pthread.h>
#include "japl_value.h"

#define JAPL_MAILBOX_DEFAULT_CAPACITY 256

typedef struct JaplMailbox {
    JaplValue* buffer;
    int capacity;
    int head;
    int tail;
    int count;
    pthread_mutex_t lock;
    pthread_cond_t not_empty;
} JaplMailbox;

JaplMailbox* japl_mailbox_new(int capacity);
void japl_mailbox_send(JaplMailbox* mb, JaplValue msg);
JaplValue japl_mailbox_receive(JaplMailbox* mb);
int japl_mailbox_try_receive(JaplMailbox* mb, JaplValue* out);
int japl_mailbox_count(JaplMailbox* mb);
void japl_mailbox_free(JaplMailbox* mb);

#endif /* JAPL_MAILBOX_H */
