#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "japl_gc.h"

JaplHeap* japl_heap_new(void) {
    JaplHeap* heap = malloc(sizeof(JaplHeap));
    if (!heap) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    heap->alloc_capacity = JAPL_GC_INITIAL_CAPACITY;
    heap->alloc_count = 0;
    heap->total_bytes = 0;
    heap->gc_threshold = JAPL_GC_THRESHOLD;
    heap->allocations = malloc(sizeof(void*) * (size_t)heap->alloc_capacity);
    heap->sizes = malloc(sizeof(size_t) * (size_t)heap->alloc_capacity);
    if (!heap->allocations || !heap->sizes) {
        fprintf(stderr, "japl: out of memory\n"); abort();
    }
    return heap;
}

void* japl_gc_alloc(JaplHeap* heap, size_t size) {
    if (heap->alloc_count >= heap->alloc_capacity) {
        heap->alloc_capacity *= 2;
        heap->allocations = realloc(heap->allocations,
                                     sizeof(void*) * (size_t)heap->alloc_capacity);
        heap->sizes = realloc(heap->sizes,
                               sizeof(size_t) * (size_t)heap->alloc_capacity);
        if (!heap->allocations || !heap->sizes) {
            fprintf(stderr, "japl: out of memory\n"); abort();
        }
    }

    void* ptr = malloc(size);
    if (!ptr) { fprintf(stderr, "japl: out of memory\n"); abort(); }

    heap->allocations[heap->alloc_count] = ptr;
    heap->sizes[heap->alloc_count] = size;
    heap->alloc_count++;
    heap->total_bytes += size;

    return ptr;
}

void japl_gc_collect(JaplHeap* heap) {
    /*
     * Placeholder: a real implementation would do mark-sweep.
     * For now this is a no-op; memory is freed on process death
     * via japl_gc_free_all.
     */
    (void)heap;
}

void japl_gc_free_all(JaplHeap* heap) {
    for (int i = 0; i < heap->alloc_count; i++) {
        free(heap->allocations[i]);
    }
    heap->alloc_count = 0;
    heap->total_bytes = 0;
}

void japl_heap_destroy(JaplHeap* heap) {
    if (heap) {
        japl_gc_free_all(heap);
        free(heap->allocations);
        free(heap->sizes);
        free(heap);
    }
}
