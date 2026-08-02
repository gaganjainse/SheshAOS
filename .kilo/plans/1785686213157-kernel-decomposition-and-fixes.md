# Kernel Decomposition and Fix Plan

## Remaining Issues (9 total)

### High Priority

1. **High #5**: Extract duplicated error-handling in `execute_task` (kernel.rs)
   - The coder failure block (lines ~270-289) and reviewer failure block (lines ~367-388) are duplicated
   - Extract into a helper method `emit_failure_and_return` (already exists, but the duplicated blocks still exist inline)
   - Replace the inline duplicated blocks with calls to the helper

2. **High #6**: Validate tool arguments in `ToolRequest` (kernel.rs)
   - Currently uses `serde_json::json!({})` for all tool arguments
   - Need to validate that the tool name is not empty and arguments are valid JSON

### Medium Priority

3. **Medium #10**: Fix `TaskInput::Multi` to preserve semantic structure
   - Currently joins all parts with newline, destroying structure
   - Should use a separator that preserves semantic boundaries (e.g., `"\n---\n"`)

4. **Medium #11**: Add error recovery for partial failures in `execute_task`
   - If planner succeeds but coder fails, task is left in inconsistent `Executing` state
   - Need to transition task to `Failed` state when partial failure occurs

5. **Medium #13**: Extract SSE parsing logic in `openai_compat.rs` into `parse_sse_buffer` function
   - The SSE parsing logic (lines ~225-236 and post-loop buffer drain) should be extracted into a standalone function

### Low Priority

6. **Low #18**: Add `SqliteEventStore` unit tests
   - Add tests for `read_for_task`, `read_since`, and `count` methods

7. **Low #19**: Make `MAX_TOOL_OUTPUT_SIZE` configurable in `AppConfig`
   - Currently hardcoded as `pub const MAX_TOOL_OUTPUT_SIZE: usize = 1_048_576`
   - Should be a field in `ResourceLimitsConfig` with a default value

8. **Low #20**: Clean up `anyhow` usage
   - Check for unnecessary `anyhow` dependencies or usage patterns

## Implementation Order
1. High #5 (extract error handling) - kernel.rs
2. High #6 (validate tool args) - kernel.rs
3. Medium #10 (TaskInput::Multi separator) - task.rs
4. Medium #11 (partial failure recovery) - kernel.rs
5. Medium #13 (extract SSE parsing) - openai_compat.rs
6. Low #18 (SqliteEventStore tests) - sqlite_event_store.rs
7. Low #19 (MAX_TOOL_OUTPUT_SIZE configurable) - config.rs, events.rs, kernel.rs
8. Low #20 (clean up anyhow) - error.rs, Cargo.toml
