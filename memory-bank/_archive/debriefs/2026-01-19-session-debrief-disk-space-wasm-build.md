---
date: 2026-01-19
type: session-debrief
tags: [debugging, disk-space, wasm, compilation, windows]
status: resolved
duration: ~15min
---

# Session Debrief: Disk Space Issue During WASM Build

## Summary

User reported "Есть ошибка во фронтенде" (There's an error in the frontend). Investigation revealed a disk space exhaustion error during WASM compilation on Windows, which was resolved by selectively cleaning the WASM target directory.

## Context

- **Project**: Leptos Marketplace (Rust full-stack app)
- **Environment**: Windows 11, PowerShell
- **Build Tool**: trunk serve (for frontend), cargo for backend
- **Issue**: Frontend compilation failing with "There is not enough space on the disk. (os error 112)"

## Timeline of Events

1. **Initial Investigation**: Checked trunk serve terminal output
2. **Root Cause Identified**: Disk space exhaustion during archive creation at `libfrontend.rlib`
3. **First Attempt**: Tried `cargo clean` - failed due to locked files (backend.exe running)
4. **Second Attempt**: Also failed with missing path errors
5. **Solution**: Cleaned only WASM target directory with PowerShell `Remove-Item`
6. **Resolution**: Successfully rebuilt frontend in 1m 53s

## Main Difficulties

### 1. Multiple Interleaved Issues
- **What caused uncertainty**: Terminal showed both disk space errors AND older compilation errors (from previous work)
- **Impact**: Initially unclear which error was the current blocker
- **Resolution**: Identified disk space error (os error 112) as the immediate issue

### 2. Locked Files
- **What caused uncertainty**: Standard `cargo clean` failed with "Access is denied"
- **Missing information**: Which process was locking the files
- **Resolution**: Inferred backend process was running; used targeted cleanup instead of full clean

### 3. Windows-Specific Paths
- **What caused uncertainty**: PowerShell path handling and error messages
- **Resolution**: Used PowerShell `Remove-Item` with proper Windows path syntax

## Resolutions

1. **Selective Cleanup Strategy**: Instead of full `cargo clean`, removed only `target\wasm32-unknown-unknown` directory
2. **PowerShell Command**: `Remove-Item -Path "target\wasm32-unknown-unknown" -Recurse -Force -ErrorAction SilentlyContinue`
3. **Targeted Rebuild**: Built only frontend for WASM target: `cargo build --target=wasm32-unknown-unknown --manifest-path crates/frontend/Cargo.toml`

## Technical Details

### Error Messages Encountered

1. **Disk Space Error**:
   ```
   error: failed to build archive at `...\libfrontend.rlib`: 
   There is not enough space on the disk. (os error 112)
   ```

2. **Locked File Error**:
   ```
   error: failed to remove file `...\backend.exe`
   Caused by: Access is denied. (os error 5)
   ```

3. **Missing Path Error**:
   ```
   error: failed to write `...\invoked.timestamp`
   Caused by: The system cannot find the path specified. (os error 3)
   ```

### Successful Approach

- **Disk Space Freed**: ~several GB by removing WASM artifacts
- **Build Time**: 1m 53s for full frontend WASM build
- **Result**: 7 warnings (unused imports only, non-critical)

## Related Notes

- [[KI_disk-space-wasm-windows_2026-01-19]] - Known issue documentation
- [[RB_recover-disk-space-rust-builds_v1]] - Runbook for future reference

## Lessons Learned

1. **Targeted Cleanup**: When `cargo clean` fails, clean specific target directories
2. **Background Processes**: Check for running processes (backend server) before attempting full cleanup
3. **Windows Specifics**: Use PowerShell commands with proper error suppression for automated cleanup
4. **WASM Artifacts**: WASM builds can consume significant disk space and should be cleaned periodically

## Open Questions / TODO

- [ ] Consider adding disk space monitoring to prevent this issue
- [ ] Document recommended minimum disk space for development
- [ ] Consider moving `target/` to a different drive with more space
- [ ] Investigate CI/CD cleanup strategies for Windows builds

## Outcome

✅ **Resolved**: Frontend builds successfully, trunk serve updated automatically
✅ **No Data Loss**: All recent refactoring work (a016, a017, a018) intact
✅ **Production Ready**: Code compiles with only minor warnings (unused imports)

## Next Steps for User

1. Monitor disk space during development
2. Run `cargo clean` (or selective cleanup) periodically when disk space is low
3. Consider configuring CARGO_TARGET_DIR to a larger drive if space constraints persist
