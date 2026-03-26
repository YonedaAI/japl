#include <assert.h>
#include <string.h>
#include "japl_runtime.h"

void test_gc_heap_create(void) {
    JaplHeap* heap = japl_heap_new();
    assert(heap != NULL);
    assert(heap->alloc_count == 0);
    assert(heap->total_bytes == 0);
    japl_heap_destroy(heap);
}

void test_gc_alloc(void) {
    JaplHeap* heap = japl_heap_new();
    void* p = japl_gc_alloc(heap, 128);
    assert(p != NULL);
    assert(heap->alloc_count == 1);
    assert(heap->total_bytes == 128);
    japl_heap_destroy(heap);
}

void test_gc_multiple_allocs(void) {
    JaplHeap* heap = japl_heap_new();
    for (int i = 0; i < 100; i++) {
        void* p = japl_gc_alloc(heap, 64);
        assert(p != NULL);
    }
    assert(heap->alloc_count == 100);
    assert(heap->total_bytes == 6400);
    japl_heap_destroy(heap);
}

void test_gc_free_all(void) {
    JaplHeap* heap = japl_heap_new();
    for (int i = 0; i < 50; i++) {
        japl_gc_alloc(heap, 32);
    }
    assert(heap->alloc_count == 50);
    japl_gc_free_all(heap);
    assert(heap->alloc_count == 0);
    assert(heap->total_bytes == 0);
    japl_heap_destroy(heap);
}

void test_gc_collect_noop(void) {
    JaplHeap* heap = japl_heap_new();
    japl_gc_alloc(heap, 100);
    japl_gc_collect(heap); /* Should not crash */
    assert(heap->alloc_count == 1);
    japl_heap_destroy(heap);
}

void test_gc_write_to_alloc(void) {
    JaplHeap* heap = japl_heap_new();
    char* buf = japl_gc_alloc(heap, 256);
    strcpy(buf, "hello from GC-tracked memory");
    assert(strcmp(buf, "hello from GC-tracked memory") == 0);
    japl_heap_destroy(heap);
}
