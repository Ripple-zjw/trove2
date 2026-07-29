## Task 2 Report

**Status:** DONE

**Commits:** 208713e

**Test result:** cargo build -p trove2 -- PASS (0 warnings)

**Files changed:**
- `src-tauri/src/lib.rs` — Added `AppState` struct with `cancel_flag` (Arc<AtomicBool>), registered `.manage(AppState{...})`, and registered `concat_videos` + `cancel_concat` commands.
- `src-tauri/src/commands/video.rs` — Added helper functions (`resolve_output_path`, `get_total_duration_us`, `check_all_same_codec`, `is_format_error`, `run_ffmpeg`, `run_concat_copy`, `run_concat_reencode`, `build_concat_result_success`, `build_concat_result_failure`) and 2 Tauri commands (`concat_videos`, `cancel_concat`).

**Self-review concerns:**
- `run_ffmpeg` reads ffmpeg's stdout (progress lines via `-progress pipe:1`) line-by-line until `progress=end`, then waits for child exit. On cancel, it kills the child and deletes the output file.
- `concat_videos` implements the full fallback strategy: try copy on same-codec inputs, fallback to reencode on format errors (`Non-monotonous DTS`, `Invalid`, `Packet mismatch`) or other errors (unless cancelled).
- Temp list file for concat demuxer is created in `concat_videos` and cleaned up immediately after `run_concat_copy` returns, avoiding leaks on error paths.
- `duration_ms` in `ConcatResult` stores elapsed wall-clock time (not video duration), matching the task's "elapsed time" requirement.
