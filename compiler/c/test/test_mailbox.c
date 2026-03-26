#include <assert.h>
#include <pthread.h>
#include "japl_runtime.h"

void test_mailbox_create(void) {
    JaplMailbox* mb = japl_mailbox_new(16);
    assert(mb != NULL);
    assert(japl_mailbox_count(mb) == 0);
    japl_mailbox_free(mb);
}

void test_mailbox_send_receive(void) {
    JaplMailbox* mb = japl_mailbox_new(16);
    japl_mailbox_send(mb, japl_int(42));
    assert(japl_mailbox_count(mb) == 1);
    JaplValue msg = japl_mailbox_receive(mb);
    assert(japl_to_int(msg) == 42);
    assert(japl_mailbox_count(mb) == 0);
    japl_mailbox_free(mb);
}

void test_mailbox_fifo(void) {
    JaplMailbox* mb = japl_mailbox_new(16);
    japl_mailbox_send(mb, japl_int(1));
    japl_mailbox_send(mb, japl_int(2));
    japl_mailbox_send(mb, japl_int(3));
    assert(japl_to_int(japl_mailbox_receive(mb)) == 1);
    assert(japl_to_int(japl_mailbox_receive(mb)) == 2);
    assert(japl_to_int(japl_mailbox_receive(mb)) == 3);
    japl_mailbox_free(mb);
}

void test_mailbox_try_receive(void) {
    JaplMailbox* mb = japl_mailbox_new(16);
    JaplValue out;
    assert(japl_mailbox_try_receive(mb, &out) == 0);
    japl_mailbox_send(mb, japl_int(7));
    assert(japl_mailbox_try_receive(mb, &out) == 1);
    assert(japl_to_int(out) == 7);
    japl_mailbox_free(mb);
}

void test_mailbox_grow(void) {
    JaplMailbox* mb = japl_mailbox_new(4);
    /* Send more than capacity */
    for (int i = 0; i < 20; i++) {
        japl_mailbox_send(mb, japl_int(i));
    }
    assert(japl_mailbox_count(mb) == 20);
    /* Receive in order */
    for (int i = 0; i < 20; i++) {
        JaplValue msg = japl_mailbox_receive(mb);
        assert(japl_to_int(msg) == i);
    }
    japl_mailbox_free(mb);
}

static void* sender_thread(void* arg) {
    JaplMailbox* mb = (JaplMailbox*)arg;
    for (int i = 0; i < 100; i++) {
        japl_mailbox_send(mb, japl_int(i));
    }
    return NULL;
}

void test_mailbox_concurrent(void) {
    JaplMailbox* mb = japl_mailbox_new(32);
    pthread_t t;
    pthread_create(&t, NULL, sender_thread, mb);

    int sum = 0;
    for (int i = 0; i < 100; i++) {
        JaplValue msg = japl_mailbox_receive(mb);
        sum += (int)japl_to_int(msg);
    }
    pthread_join(t, NULL);
    /* Sum of 0..99 = 4950 */
    assert(sum == 4950);
    japl_mailbox_free(mb);
}
