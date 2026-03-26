#ifndef JAPL_GC_H
#define JAPL_GC_H

#include <stddef.h>

#define JAPL_GC_INITIAL_CAPACITY 256
#define JAPL_GC_THRESHOLD (1024 * 1024)  /* 1 MB */

typedef struct JaplHeap {
    void** allocations;
    size_t* sizes;
    int alloc_count;
    int alloc_capacity;
    size_t total_bytes;
    size_t gc_threshold;
} JaplHeap;

/* Create a new heap. */
JaplHeap* japl_heap_new(void);

/* Allocate memory tracked by the heap. */
void* japl_gc_alloc(JaplHeap* heap, size_t size);

/* Collect garbage (placeholder — simple free-all for now). */
void japl_gc_collect(JaplHeap* heap);

/* Free all allocations (on process death). */
void japl_gc_free_all(JaplHeap* heap);

/* Destroy the heap itself. */
void japl_heap_destroy(JaplHeap* heap);

#endif /* JAPL_GC_H */
