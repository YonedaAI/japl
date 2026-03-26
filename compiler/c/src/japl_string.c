#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "japl_string.h"

JaplValue japl_string(const char* s) {
    JaplString* str = malloc(sizeof(JaplString));
    if (!str) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    str->length = (int)strlen(s);
    str->data = malloc((size_t)str->length + 1);
    if (!str->data) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    memcpy(str->data, s, (size_t)str->length + 1);
    str->ref_count = 1;

    JaplValue val;
    val.kind = JAPL_STRING;
    val.string_val = str;
    return val;
}

JaplValue japl_string_concat(JaplValue a, JaplValue b) {
    if (a.kind != JAPL_STRING || b.kind != JAPL_STRING) {
        fprintf(stderr, "japl: string_concat on non-string values\n");
        abort();
    }
    int new_len = a.string_val->length + b.string_val->length;
    JaplString* str = malloc(sizeof(JaplString));
    if (!str) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    str->length = new_len;
    str->data = malloc((size_t)new_len + 1);
    if (!str->data) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    memcpy(str->data, a.string_val->data, (size_t)a.string_val->length);
    memcpy(str->data + a.string_val->length, b.string_val->data,
           (size_t)b.string_val->length + 1);
    str->ref_count = 1;

    JaplValue val;
    val.kind = JAPL_STRING;
    val.string_val = str;
    return val;
}

int japl_string_length(JaplValue s) {
    if (s.kind != JAPL_STRING) {
        fprintf(stderr, "japl: string_length on non-string value\n");
        abort();
    }
    return s.string_val->length;
}

void japl_string_retain(JaplString* s) {
    if (s) s->ref_count++;
}

void japl_string_release(JaplString* s) {
    if (s) {
        s->ref_count--;
        if (s->ref_count <= 0) {
            free(s->data);
            free(s);
        }
    }
}
