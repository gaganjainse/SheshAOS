#ifndef NEXUSAOS_SCHEDULER_H
#define NEXUSAOS_SCHEDULER_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Scheduler Scheduler;
typedef struct SchedulerEntry SchedulerEntry;

// Create a new scheduler with a maximum queue depth.
Scheduler* scheduler_create(size_t max_depth);

// Enqueue a task. Returns false if queue is full.
bool scheduler_enqueue(Scheduler* s, const uint8_t* task_id, uint8_t priority);

// Dequeue the highest-priority task. Returns NULL if empty.
const SchedulerEntry* scheduler_dequeue(Scheduler* s);

// Get current queue depth.
size_t scheduler_queue_depth(const Scheduler* s);

// Cancel a task by task_id bytes (16 bytes UUID).
// Returns true if found and cancelled.
bool scheduler_cancel(Scheduler* s, const uint8_t* task_id);

// Destroy and free the scheduler.
void scheduler_deinit(Scheduler* s);

#ifdef __cplusplus
}
#endif

#endif // NEXUSAOS_SCHEDULER_H