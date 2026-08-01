#ifndef VT100_EXPERT_H
#define VT100_EXPERT_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct VT100Parser VT100Parser;

// Create a new high-speed C VT100 ANSI byte parser
VT100Parser* vt100_parser_create(size_t cols, size_t rows);

// Feed raw terminal stdout bytes to the parser (zero-copy processing)
void vt100_parser_feed(VT100Parser* parser, const uint8_t* data, size_t len);

// Get total lines processed
size_t vt100_parser_get_line_count(const VT100Parser* parser);

// Clean up memory
void vt100_parser_free(VT100Parser* parser);

#ifdef __cplusplus
}
#endif

#endif // VT100_EXPERT_H
