#ifndef JAPL_MAP_H
#define JAPL_MAP_H

#include "japl_value.h"

#define JAPL_MAP_INITIAL_CAPACITY 16
#define JAPL_MAP_LOAD_FACTOR 0.75

typedef struct JaplMapEntry {
    const char* key;
    JaplValue value;
    int occupied;
} JaplMapEntry;

typedef struct JaplMap {
    JaplMapEntry* entries;
    int capacity;
    int count;
} JaplMap;

JaplMap* japl_map_new(void);
void japl_map_free(JaplMap* map);
void japl_map_put(JaplMap* map, const char* key, JaplValue value);
JaplValue* japl_map_get(JaplMap* map, const char* key);
int japl_map_contains(JaplMap* map, const char* key);
void japl_map_remove(JaplMap* map, const char* key);
int japl_map_count(JaplMap* map);

#endif /* JAPL_MAP_H */
