#ifndef NEXUSAOS_EVENT_STORE_H
#define NEXUSAOS_EVENT_STORE_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct EventStore EventStore;

// Create a new event store at the given JSONL file path.
// Returns NULL on failure.
EventStore* event_store_open(const char* path);

// Append a JSON event line to the store.
// Returns true on success, false on failure.
bool event_store_append(EventStore* store, const char* json_line);

// Get total event count.
uint64_t event_store_count(EventStore* store);

// Get the next sequence number and increment internally.
uint64_t event_store_next_seq(EventStore* store);

// Destroy and free the event store.
void event_store_deinit(EventStore* store);

#ifdef __cplusplus
}
#endif

#endif // NEXUSAOS_EVENT_STORE_H