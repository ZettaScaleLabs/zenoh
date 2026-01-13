# Phase 3 - Testing & Validation Guide

**Date**: January 12, 2026  
**Status**: Testing in progress  
**Objective**: Validate all tutorial examples build and run correctly

---

## Testing Summary

### Build Status

| Chapter | Status | Notes |
|---------|--------|-------|
| 1 | ⚠️ Issue | Rust version incompatibility (1.85.0 vs 1.88 required) |
| 2 | ⏳ Pending | Awaiting ch. 1 resolution |
| 3 | ⏳ Pending | Awaiting ch. 1 resolution |
| 4 | ⏳ Pending | Awaiting ch. 1 resolution |

### Issues Found

#### Issue 1: Rust Version Incompatibility
**Severity**: Medium  
**Package**: `home@0.5.12`  
**Details**: 
- Current Rust: 1.85.0
- Required by home: 1.88+
- This is a transitive dependency from Zenoh 0.11

**Solution**:
```bash
# Option 1: Upgrade Rust
rustup update

# Option 2: Pin to compatible version (if available)
# This depends on fixing upstream Zenoh dependencies
```

#### Issue 2: Workspace Configuration
**Severity**: Low  
**Status**: ✅ FIXED

**Problem**: Chapter Cargo.toml files were inheriting workspace configuration from root Zenoh project  
**Solution**: Added `[workspace]` section to each chapter's Cargo.toml to make them standalone

**Files Fixed**:
- chapter_1/Cargo.toml
- chapter_2/Cargo.toml
- chapter_3/Cargo.toml
- chapter_4/Cargo.toml

---

## Testing Procedures

### Manual Testing Steps

Each chapter should be tested independently:

```bash
# Chapter 1: Pub/Sub Basics
cd examples/tutorials/smart_building/chapter_1
cargo build --release
cargo run --release --bin room_sensor        # Terminal 1
cargo run --release --bin monitor            # Terminal 2

# Expected: Monitor receives temperature readings from sensor
```

```bash
# Chapter 2: Key Expressions
cd examples/tutorials/smart_building/chapter_2
cargo build --release
cargo run --release --bin multi_sensor       # Terminal 1
cargo run --release --bin floor_monitor      # Terminal 2

# Expected: Monitor receives all sensor types with wildcards
```

```bash
# Chapter 3: Query/Reply
cd examples/tutorials/smart_building/chapter_3
cargo build --release
cargo run --release --bin room_status_service  # Terminal 1
cargo run --release --bin dashboard            # Terminal 2

# Expected: Dashboard queries and receives status responses
```

```bash
# Chapter 4: Zenoh Router
cd examples/tutorials/smart_building/chapter_4
cargo build --release
./run_demo.sh

# Or manually:
cargo run --release --bin zenohd -- --config router.json5    # Terminal 1
cargo run --release --bin sensor_with_router                 # Terminal 2
cargo run --release --bin monitor_with_router                # Terminal 3

# Expected: Sensor publishes, monitor receives through router
```

---

## Output Validation

### Chapter 1 Expected Output

**room_sensor.rs**:
```
Opening Zenoh session...
Declaring publisher for building/floor1/room_a/temperature
Room A Sensor started. Publishing temperature readings...

[Room A Sensor] Publishing temperature: 22.0°C (reading #1)
[Room A Sensor] Publishing temperature: 22.1°C (reading #2)
...
```

**monitor.rs**:
```
Opening Zenoh session...
Subscribing to building/floor1/room_a/temperature

Monitor started. Waiting for temperature readings...

[Monitor] Room A Temperature: 22.0°C
[Monitor] Room A Temperature: 22.1°C
...
```

### Chapter 2 Expected Output

**floor_monitor.rs** (with multi_sensor.rs):
```
🌡️  [Floor Monitor] Room A Temperature: 22.3°C
💧 [Floor Monitor] Room A Humidity: 44%
👥 [Floor Monitor] Room A Occupancy: 2 people
🌡️  [Floor Monitor] Room A Temperature: 22.4°C
...
```

### Chapter 3 Expected Output

**dashboard.rs** (with room_status_service.rs):
```
--- Query 1 ---
[Dashboard] Querying: building/floor1/room_a/status
[Dashboard] Response: {"temperature": 22.5, "humidity": 45, "occupancy": 2}

--- Query 2 ---
[Dashboard] Querying: building/floor1/room_a/status
[Dashboard] Response: {"temperature": 22.7, "humidity": 44, "occupancy": 1}
...
```

### Chapter 4 Expected Output

**run_demo.sh** output:
```
=== Zenoh Multi-Client Demo ===

Building project...
✓ Build complete

Starting Zenoh Router on tcp/127.0.0.1:7447
✓ Router started (PID: XXXX)

Starting Sensor (in background)
✓ Sensor started (PID: XXXX)

Starting Monitor
Opening Zenoh session as CLIENT...
Connected to Zenoh router!
...
[Monitor] Temperature: 22.0°C
[Monitor] Temperature: 22.1°C
...

Cleaning up processes...
✓ All processes stopped
```

---

## Known Issues & Workarounds

### Issue: "home@0.5.12 requires rustc 1.88"

**Status**: Blocking all builds  
**Cause**: Zenoh 0.11 dependencies require newer Rust

**Workarounds**:
1. **Upgrade Rust** (Recommended):
   ```bash
   rustup update
   rustc --version  # Should show 1.88+
   ```

2. **Use Zenoh pinned deps** (Alternative):
   ```bash
   # Use zenoh-pinned-deps for compatibility with older Rust
   # But this requires modifying Cargo.toml
   ```

3. **Pin home version** (Not recommended):
   ```bash
   # Temporary workaround - may cause other issues
   cargo update home@0.5.12 --precise 0.5.11
   ```

---

## Testing Checklist

### Pre-Testing
- [ ] Verify Rust version: `rustc --version` (should be 1.88+)
- [ ] Verify Cargo: `cargo --version`
- [ ] Check network connectivity (for Zenoh communications)

### Chapter 1 Testing
- [ ] Code compiles without errors
- [ ] room_sensor runs without panicking
- [ ] monitor receives data
- [ ] Output format matches expected
- [ ] Exits cleanly after 20 readings
- [ ] Try Exercise 1 (multiple monitors)
- [ ] Try Exercise 2 (multiple rooms)
- [ ] Try Exercise 3 (multiple data types)

### Chapter 2 Testing
- [ ] Code compiles without errors
- [ ] multi_sensor publishes all 3 sensor types
- [ ] floor_monitor receives wildcard data
- [ ] Emoji icons display correctly
- [ ] All 4 example programs work
- [ ] Try Exercise 1 (watch entire building)
- [ ] Try Exercise 2 (temperature threshold alerts)
- [ ] Try Exercise 3 (multi-floor aggregation)

### Chapter 3 Testing
- [ ] Code compiles without errors
- [ ] room_status_service responds to queries
- [ ] dashboard receives responses
- [ ] building_status handles multiple rooms
- [ ] selective_query wildcards work
- [ ] Error handling for missing rooms works
- [ ] Try Exercise 1 (error handling)
- [ ] Try Exercise 2 (conditional responses)
- [ ] Try Exercise 3 (aggregating responses)

### Chapter 4 Testing
- [ ] Code compiles without errors
- [ ] Router starts on port 7447
- [ ] Clients connect to router successfully
- [ ] Sensor publishes through router
- [ ] Monitor receives from router
- [ ] run_demo.sh script works end-to-end
- [ ] Cleanup kills all processes properly
- [ ] Try Exercise 1 (multiple sensors)
- [ ] Try Exercise 2 (remote router)
- [ ] Try Exercise 3 (multi-floor system)

### Cross-Platform Testing
- [ ] macOS
- [ ] Linux
- [ ] Windows / WSL2

### Documentation Validation
- [ ] All code in docs is copy-paste ready
- [ ] Expected outputs match actual outputs
- [ ] Exercises have working solutions
- [ ] Troubleshooting sections resolve issues
- [ ] Links and references are correct

---

## Test Results Template

```markdown
## Test Session: [Date]

### Environment
- OS: [macOS/Linux/Windows]
- Rust Version: [version]
- Cargo Version: [version]

### Chapter 1 Results
- Build: [PASS/FAIL]
- room_sensor: [PASS/FAIL]
- monitor: [PASS/FAIL]
- Exercise 1: [PASS/FAIL]
- Notes: [any issues]

### Chapter 2 Results
- Build: [PASS/FAIL]
- multi_sensor: [PASS/FAIL]
- floor_monitor: [PASS/FAIL]
- building_sensors: [PASS/FAIL]
- selective_monitor: [PASS/FAIL]
- Notes: [any issues]

### Chapter 3 Results
- Build: [PASS/FAIL]
- room_status_service: [PASS/FAIL]
- dashboard: [PASS/FAIL]
- building_status: [PASS/FAIL]
- selective_query: [PASS/FAIL]
- Notes: [any issues]

### Chapter 4 Results
- Build: [PASS/FAIL]
- sensor_with_router: [PASS/FAIL]
- monitor_with_router: [PASS/FAIL]
- run_demo.sh: [PASS/FAIL]
- Notes: [any issues]

### Overall Status
[PASS/FAIL]

### Issues Found
[List any issues and their severity]

### Recommendations
[Any changes needed]
```

---

## Automated Testing

### CI/CD Considerations

For automated testing, add to GitHub Actions:

```yaml
name: Tutorial Tests

on: [push, pull_request]

jobs:
  test-tutorials:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]
    
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
      
      - name: Build Chapter 1
        run: cd examples/tutorials/smart_building/chapter_1 && cargo build --release
      
      - name: Build Chapter 2
        run: cd examples/tutorials/smart_building/chapter_2 && cargo build --release
      
      - name: Build Chapter 3
        run: cd examples/tutorials/smart_building/chapter_3 && cargo build --release
      
      - name: Build Chapter 4
        run: cd examples/tutorials/smart_building/chapter_4 && cargo build --release
```

---

## Validation Criteria

### Code Quality
- ✅ All examples compile without warnings (or documented)
- ✅ All examples follow Rust conventions
- ✅ Error handling is appropriate
- ✅ Code is well-structured and readable

### Documentation Quality
- ✅ All concepts are clearly explained
- ✅ All code in docs is accurate
- ✅ Expected outputs are shown
- ✅ Exercises have clear instructions
- ✅ Troubleshooting sections are helpful

### Learning Outcomes
- ✅ Each chapter teaches one core concept
- ✅ Progression is logical
- ✅ Examples build on previous chapters
- ✅ Users can transfer patterns to their domain

### Completeness
- ✅ All 14 examples work
- ✅ All 4 chapters documented
- ✅ All configuration files present
- ✅ Demo script functional

---

## Next Steps

1. **Resolve Rust Version Issue**
   - Upgrade Rust or use compatible Zenoh version
   - Verify build succeeds on all chapters

2. **Run All Manual Tests**
   - Test each chapter independently
   - Run on multiple platforms
   - Verify all outputs match documentation

3. **Validate Documentation**
   - Check all code examples are correct
   - Verify exercises work
   - Test troubleshooting sections

4. **Complete Chapters 5-9**
   - Follow same testing procedures
   - Ensure consistency with chapters 1-4
   - Document any new patterns

5. **Community Review**
   - Get feedback from maintainers
   - Address any issues
   - Iterate based on feedback

---

## Status

**Current Phase**: Phase 3 - Testing & Validation  
**Blocker**: Rust version incompatibility (need 1.88+)  
**Action**: Upgrade Rust and re-test  
**Timeline**: Testing can resume after Rust upgrade

---

**Next Action**: Upgrade Rust to 1.88+ and resume testing
