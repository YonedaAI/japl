#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "japl_map.h"

static unsigned int hash_string(const char* key, int capacity) {
    unsigned int hash = 5381;
    while (*key) {
        hash = ((hash << 5) + hash) + (unsigned char)*key;
        key++;
    }
    return hash % (unsigned int)capacity;
}

static void japl_map_resize(JaplMap* map) {
    int old_capacity = map->capacity;
    JaplMapEntry* old_entries = map->entries;

    map->capacity *= 2;
    map->entries = calloc((size_t)map->capacity, sizeof(JaplMapEntry));
    if (!map->entries) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    map->count = 0;

    for (int i = 0; i < old_capacity; i++) {
        if (old_entries[i].occupied) {
            japl_map_put(map, old_entries[i].key, old_entries[i].value);
        }
    }
    free(old_entries);
}

JaplMap* japl_map_new(void) {
    JaplMap* map = malloc(sizeof(JaplMap));
    if (!map) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    map->capacity = JAPL_MAP_INITIAL_CAPACITY;
    map->count = 0;
    map->entries = calloc((size_t)map->capacity, sizeof(JaplMapEntry));
    if (!map->entries) { fprintf(stderr, "japl: out of memory\n"); abort(); }
    return map;
}

void japl_map_free(JaplMap* map) {
    if (map) {
        free(map->entries);
        free(map);
    }
}

void japl_map_put(JaplMap* map, const char* key, JaplValue value) {
    if ((double)map->count / (double)map->capacity >= JAPL_MAP_LOAD_FACTOR) {
        japl_map_resize(map);
    }

    unsigned int idx = hash_string(key, map->capacity);
    for (int i = 0; i < map->capacity; i++) {
        unsigned int probe = (idx + (unsigned int)i) % (unsigned int)map->capacity;
        if (!map->entries[probe].occupied) {
            map->entries[probe].key = key;
            map->entries[probe].value = value;
            map->entries[probe].occupied = 1;
            map->count++;
            return;
        }
        if (strcmp(map->entries[probe].key, key) == 0) {
            map->entries[probe].value = value;
            return;
        }
    }
}

JaplValue* japl_map_get(JaplMap* map, const char* key) {
    unsigned int idx = hash_string(key, map->capacity);
    for (int i = 0; i < map->capacity; i++) {
        unsigned int probe = (idx + (unsigned int)i) % (unsigned int)map->capacity;
        if (!map->entries[probe].occupied) return NULL;
        if (strcmp(map->entries[probe].key, key) == 0) {
            return &map->entries[probe].value;
        }
    }
    return NULL;
}

int japl_map_contains(JaplMap* map, const char* key) {
    return japl_map_get(map, key) != NULL;
}

void japl_map_remove(JaplMap* map, const char* key) {
    unsigned int idx = hash_string(key, map->capacity);
    for (int i = 0; i < map->capacity; i++) {
        unsigned int probe = (idx + (unsigned int)i) % (unsigned int)map->capacity;
        if (!map->entries[probe].occupied) return;
        if (strcmp(map->entries[probe].key, key) == 0) {
            map->entries[probe].occupied = 0;
            map->entries[probe].key = NULL;
            map->count--;
            /* Rehash subsequent entries to fix linear probing gaps */
            unsigned int next = (probe + 1) % (unsigned int)map->capacity;
            while (map->entries[next].occupied) {
                JaplMapEntry tmp = map->entries[next];
                map->entries[next].occupied = 0;
                map->entries[next].key = NULL;
                map->count--;
                japl_map_put(map, tmp.key, tmp.value);
                next = (next + 1) % (unsigned int)map->capacity;
            }
            return;
        }
    }
}

int japl_map_count(JaplMap* map) {
    return map->count;
}
