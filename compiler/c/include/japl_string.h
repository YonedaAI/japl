#ifndef JAPL_STRING_H
#define JAPL_STRING_H

#include "japl_value.h"

typedef struct JaplString {
    char* data;
    int length;
    int ref_count;
} JaplString;

/* Create a JaplValue wrapping a new immutable string. */
JaplValue japl_string(const char* s);

/* Concatenate two string values. */
JaplValue japl_string_concat(JaplValue a, JaplValue b);

/* Return the length of a string value. */
int japl_string_length(JaplValue s);

/* Increment reference count. */
void japl_string_retain(JaplString* s);

/* Decrement reference count; frees when it hits 0. */
void japl_string_release(JaplString* s);

#endif /* JAPL_STRING_H */
