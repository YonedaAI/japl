#include <stdio.h>
#include <stdlib.h>
#include "japl_list.h"

JaplValue japl_cons(JaplValue head, JaplValue tail) {
    if (tail.kind != JAPL_NIL && tail.kind != JAPL_LIST) {
        fprintf(stderr, "japl: cons tail must be a list or nil\n");
        abort();
    }

    JaplList* node = malloc(sizeof(JaplList));
    if (!node) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    node->head = head;

    if (tail.kind == JAPL_NIL) {
        node->tail = NULL;
        node->length = 1;
    } else {
        node->tail = tail.list_val;
        node->length = tail.list_val->length + 1;
    }

    JaplValue val;
    val.kind = JAPL_LIST;
    val.list_val = node;
    return val;
}

JaplValue japl_head(JaplValue list) {
    if (list.kind != JAPL_LIST || list.list_val == NULL) {
        fprintf(stderr, "japl: head on empty list\n");
        abort();
    }
    return list.list_val->head;
}

JaplValue japl_tail(JaplValue list) {
    if (list.kind != JAPL_LIST || list.list_val == NULL) {
        fprintf(stderr, "japl: tail on empty list\n");
        abort();
    }
    if (list.list_val->tail == NULL) {
        return japl_nil();
    }
    JaplValue val;
    val.kind = JAPL_LIST;
    val.list_val = list.list_val->tail;
    return val;
}

int japl_list_length(JaplValue list) {
    if (list.kind == JAPL_NIL) return 0;
    if (list.kind != JAPL_LIST) {
        fprintf(stderr, "japl: list_length on non-list value\n");
        abort();
    }
    return list.list_val->length;
}

void japl_list_free(JaplList* list) {
    free(list);
}
