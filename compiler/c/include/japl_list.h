#ifndef JAPL_LIST_H
#define JAPL_LIST_H

#include "japl_value.h"

typedef struct JaplList {
    JaplValue head;
    struct JaplList* tail;
    int length;
} JaplList;

/* Cons a head value onto a list (or nil) tail. */
JaplValue japl_cons(JaplValue head, JaplValue tail);

/* Return the head of a list. */
JaplValue japl_head(JaplValue list);

/* Return the tail of a list (another list or nil). */
JaplValue japl_tail(JaplValue list);

/* Return the length of a list. */
int japl_list_length(JaplValue list);

/* Free a list node (not recursive — GC handles the rest). */
void japl_list_free(JaplList* list);

#endif /* JAPL_LIST_H */
