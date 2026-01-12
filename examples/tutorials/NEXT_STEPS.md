# Next Steps for Phase 3.2 - Runtime Testing

**Current Status:** Phase 3.0-3.1 Complete ✅ | All 14 binaries compile successfully

**Next Phase:** Phase 3.2 - Runtime Testing

---

## Quick Start: Run Your First Example

### 1. Test Chapter 1 (Pub/Sub) in Two Terminals

**Terminal 1 - Publisher:**
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building/chapter_1
./target/debug/room_sensor
```

Expected output:
```
Opening Zenoh session...
Declaring publisher for building/floor1/room_a/temperature
Room A Sensor started. Publishing temperature readings...

[Room A Sensor] Publishing temperature: 22.1°C (reading #1)
[Room A Sensor] Publishing temperature: 22.3°C (reading #2)
...
```

**Terminal 2 - Subscriber:**
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building/chapter_1
./target/debug/monitor
```

Expected output (after publisher starts):
```
Opening Zenoh session...
Subscribing to building/floor1/room_a/temperature

Monitor started. Waiting for temperature readings...

[Monitor] Room A Temperature: 22.1°C
[Monitor] Room A Temperature: 22.3°C
...
```

---

## Phase 3.2 Testing Plan (Detailed)

### Test 1: Chapter 1 - Basic Pub/Sub
**Duration:** ~5 minutes

**Setup:**
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building/chapter_1
```

**Run:**
1. Start publisher: `./target/debug/room_sensor &`
2. Start subscriber: `./target/debug/monitor`
3. Let run for ~30 seconds
4. Kill publisher: `fg` then `Ctrl+C`
5. Stop subscriber: `Ctrl+C`

**Verify:**
- [ ] Both processes start without errors
- [ ] Publisher publishes ~15-20 readings
- [ ] Subscriber receives all published messages
- [ ] Temperature values are numerical (22.x format)
- [ ] No panic or runtime errors

**Documentation:**
- Record actual vs expected output
- Note any timing differences
- Compare with Chapter 1 README expectations

---

### Test 2: Chapter 2 - Key Expressions & Wildcards
**Duration:** ~10 minutes

**Setup:**
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building/chapter_2
```

**Test 2a: Multi-Sensor (Non-wildcard)**
```bash
./target/debug/multi_sensor &
sleep 2
./target/debug/floor_monitor
```
Expected: Single monitor receives from 3 sensors (temperature, humidity, occupancy)

**Test 2b: Building Sensors (Multi-room)**
```bash
./target/debug/building_sensors &
sleep 2
./target/debug/selective_monitor
```
Expected: Selective monitor only receives temperature readings

**Verify:**
- [ ] All 4 programs compile and run
- [ ] Correct publishers/subscribers start
- [ ] Messages flow from pub to sub
- [ ] Wildcard patterns work correctly
- [ ] No errors in logs

---

### Test 3: Chapter 3 - Query/Reply Pattern
**Duration:** ~10 minutes

**Setup:**
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building/chapter_3
```

**Test 3a: Room Status Service**
```bash
./target/debug/room_status_service &
sleep 2
./target/debug/dashboard
```
Expected: Dashboard receives JSON status responses with temperature, humidity, occupancy

**Test 3b: Building Status Query**
```bash
./target/debug/building_status &
sleep 2
./target/debug/selective_query
```
Expected: Query client receives responses from multiple queryables

**Verify:**
- [ ] Queryables start and listen
- [ ] Queries are sent successfully
- [ ] Responses are received with correct data
- [ ] JSON format is valid (if applicable)
- [ ] No timeout errors

---

### Test 4: Chapter 4 - Router Integration
**Duration:** ~15 minutes

**Setup:**
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building/chapter_4
```

**Start Router (Terminal 1):**
```bash
zenohd -c router.json5
```
(Note: This requires zenohd binary in PATH. If not available, skip to next test or build from source)

**Publisher via Router (Terminal 2):**
```bash
./target/debug/sensor_with_router
```

**Monitor via Router (Terminal 3):**
```bash
./target/debug/monitor_with_router
```

**Verify:**
- [ ] Router starts and binds to port
- [ ] Publisher connects and publishes
- [ ] Monitor connects and receives messages
- [ ] Communication works through router
- [ ] No connection errors

**Alternative (No Router):**
If zenohd is not available, these programs can still demonstrate local pub/sub:
- Publisher will use default local endpoint
- Subscriber will connect via default endpoint

---

## Documentation Validation (Phase 3.3)

After runtime testing, validate documentation:

### Check 1: Code Examples Match Implementation
```bash
# For each chapter, verify code snippets in README.md match actual .rs files
diff <(grep -A 10 "room_sensor" examples/tutorials/smart_building/chapter_1/README.md) \
     examples/tutorials/smart_building/chapter_1/src/bin/room_sensor.rs
```

### Check 2: Expected Output Matches Actual
- [ ] Compare expected output in README with actual program output
- [ ] Document any differences
- [ ] Update documentation if needed

### Check 3: Exercises Work
- [ ] Try all chapter exercises
- [ ] Verify solutions compile
- [ ] Document exercise results

---

## Troubleshooting Guide

### Issue: "Address already in use"
**Cause:** Previous process still holding port
**Solution:** 
```bash
# Kill by name (process must not be a child)
pkill -f room_sensor
pkill -f monitor
sleep 1  # Wait for port release
```

### Issue: "Failed to open Zenoh session"
**Cause:** Possible permission or network issue
**Solution:**
```bash
# Check if Zenoh can initialize
zenoh --version
# Check environment
env | grep ZENOH
```

### Issue: No output from subscriber
**Cause:** Publisher not started, wrong key expression, or timing issue
**Solution:**
1. Ensure publisher starts first
2. Wait 2 seconds before starting subscriber
3. Check key expression matches
4. Try with shorter timeout: `timeout 10 ./target/debug/monitor`

### Issue: Binary not found
**Cause:** Not compiled or wrong directory
**Solution:**
```bash
# Rebuild
cargo build --release
# Check binary exists
ls -la target/debug/room_sensor
```

---

## Quick Reference Commands

### Build All Chapters
```bash
cd /Users/diogomatsubara/code/zettascale/zenoh/examples/tutorials/smart_building
for ch in chapter_{1,2,3,4}; do (cd "$ch" && cargo build); done
```

### Test Publisher/Subscriber Pattern
```bash
cd chapter_1
./target/debug/room_sensor > pub.log 2>&1 &
sleep 2
./target/debug/monitor > sub.log 2>&1
pkill -f room_sensor
grep "Publishing\|Received" pub.log sub.log
```

### List All Binaries
```bash
for ch in chapter_{1,2,3,4}; do
  echo "$ch:"
  ls -1 "$ch/target/debug" | grep -E "^[a-z_]+$" | grep -v "^\."
done
```

### Show Output Differences
```bash
# Run with output capture
timeout 5 ./target/debug/room_sensor > expected.txt 2>&1 || true
# Compare with documentation expectation
diff expected.txt <(grep "Publishing" ../README.md)
```

---

## Documentation References

### Understanding the Code
- **API Reference:** See `/examples/tutorials/smart_building/QUICK_REFERENCE.md`
- **API Changes:** See `/examples/tutorials/PHASE3_UPGRADE_SUMMARY.md`
- **Full Guide:** See `/examples/tutorials/PHASE3_TESTING_GUIDE.md`

### Understanding Zenoh Concepts
- **Chapter 1:** `/examples/tutorials/smart_building/chapter_1/README.md` (Pub/Sub)
- **Chapter 2:** `/examples/tutorials/smart_building/chapter_2/README.md` (Key Expressions)
- **Chapter 3:** `/examples/tutorials/smart_building/chapter_3/README.md` (Query/Reply)
- **Chapter 4:** `/examples/tutorials/smart_building/chapter_4/README.md` (Router)

---

## Expected Outcomes

### After Phase 3.2 (Runtime Testing)
- [ ] All 14 programs execute without crashes
- [ ] Pub/sub communication verified
- [ ] Query/reply pattern verified
- [ ] Router integration tested
- [ ] Actual outputs documented
- [ ] Any differences from documentation noted

### After Phase 3.3 (Documentation Validation)
- [ ] Code examples match implementation
- [ ] Expected outputs match actual
- [ ] All exercises verified to work
- [ ] Documentation updated if needed
- [ ] Tutorial ready for users

### After Phase 3.4 (Cross-Platform - Optional)
- [ ] Tests run on Linux (if available)
- [ ] Tests run on Windows (if available)
- [ ] Platform-specific notes documented

---

## Estimated Timeline

- **Phase 3.2:** 2-3 hours (4 test scenarios, documentation)
- **Phase 3.3:** 1-2 hours (validation, updates)
- **Phase 3.4:** 1-2 hours (cross-platform, optional)

---

## Key Success Criteria

✅ **All 14 programs execute without errors**
✅ **Pub/sub communication works end-to-end**
✅ **Query/reply pattern functions correctly**
✅ **Documentation matches actual behavior**
✅ **Exercises are solvable**

Once these criteria are met, the tutorial is **ready for users** and Phase 4 (Advanced Chapters 5-9) can begin.

---

## Questions?

Refer to:
1. `STATUS.md` - Current overall status
2. `PHASE3_SESSION_SUMMARY.md` - What was just completed
3. `PHASE3_UPGRADE_SUMMARY.md` - API changes made
4. Individual chapter READMEs - Specific concept details
5. `QUICK_REFERENCE.md` - Common patterns and troubleshooting
