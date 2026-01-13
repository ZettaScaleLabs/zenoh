# AGENTS.md

This document describes how AI agents can work on and improve the Zenoh Smart Building Tutorial codebase.

## Overview

The Zenoh Smart Building Tutorial is a comprehensive learning system with 9 chapters, 30+ working code examples, 4 domain adaptation guides, and extensive documentation. Agents can help maintain, enhance, and extend this codebase.

## Codebase Structure

```
examples/tutorials/
├── smart_building/
│   ├── chapter_1/          # Pub/Sub Basics
│   ├── chapter_2/          # Key Expressions & Wildcards
│   ├── chapter_3/          # Query/Reply Pattern
│   ├── chapter_4/          # Routers & Bridging
│   ├── chapter_5/          # Aggregation & Advanced Patterns
│   ├── chapter_6-9/        # Placeholders for future chapters
│   └── README.md
├── AGENTS.md               # This file
├── DOMAIN_ADAPTATIONS.md   # Domain guide navigation
├── DOMAIN_ADAPTATION_*.md  # 4 domain-specific guides
├── INDEX.md                # Tutorial navigation
├── START_HERE.md           # Entry point
├── LINK_VERIFICATION_REPORT.md
└── README.md
```

## What Agents Can Do

### Code Quality & Maintenance

- **Fix compiler warnings**: Identify and resolve Rust compiler warnings in examples
- **Update dependencies**: Keep Zenoh and other crates to latest versions
- **Refactor code**: Improve code clarity, performance, and best practices
- **Add missing error handling**: Enhance robustness of examples
- **Format code**: Ensure consistent style across all examples

### Documentation Improvements

- **Fix broken links**: Validate and repair documentation references
- **Update examples**: Synchronize documentation with actual code
- **Add clarifications**: Improve confusing sections with better explanations
- **Create new sections**: Add missing documentation based on feedback
- **Verify accuracy**: Ensure all code snippets compile and work correctly

### Testing & Validation

- **Run examples**: Verify all code examples compile and execute
- **Create tests**: Add unit tests and integration tests where missing
- **Performance testing**: Measure and optimize example performance
- **Cross-platform testing**: Ensure examples work on Linux, macOS, Windows

### Content Expansion

- **Add exercises**: Create progressively harder problems for learners
- **Extend chapters**: Develop content for chapters 6-9
- **Create domain guides**: Expand existing 4 domain guides with more examples
- **Add troubleshooting**: Document common issues and solutions

### Analytics & Feedback

- **Analyze complexity**: Measure learning curve progression
- **Identify gaps**: Find missing concepts or unclear explanations
- **Suggest improvements**: Recommend enhancements based on patterns

## How to Work with This Codebase

### Prerequisites

- Rust 1.75+ with toolchain installed
- Zenoh 1.7.2 (via crates.io, not local path dependencies)
- Git for version control
- Basic understanding of Zenoh pub/sub architecture

### Building the Tutorial

```bash
# Build all chapters
cd examples/tutorials/smart_building

# Build specific chapter
cd chapter_1
cargo build --bins

# Run examples
cargo run --bin room_sensor
cargo run --bin monitor

# Clean build
cargo clean && cargo build --bins
```

### File Organization

**Chapter Structure** (each chapter_N directory):
```
chapter_N/
├── Cargo.toml              # Dependencies - ALWAYS use Zenoh 1.7.2
├── README.md               # Tutorial content and step-by-step guide
├── src/
│   └── bin/
│       ├── example1.rs
│       ├── example2.rs
│       └── example3.rs
└── target/                 # Build artifacts (git ignored)
```

**Documentation Files** (in examples/tutorials/):
- `README.md` - Main tutorial index
- `START_HERE.md` - First-time user entry point
- `INDEX.md` - Comprehensive navigation
- `DOMAIN_ADAPTATIONS.md` - Domain guide hub
- `DOMAIN_ADAPTATION_*.md` - 4 specific domain guides
- `AGENTS.md` - This file

### Key Requirements for Code Changes

1. **Version Consistency**: Always use `zenoh = { version = "1.7.2", features = ["default"] }`
   - Do NOT use local path dependencies: `path = "../../../../zenoh"`
   - Do NOT use version ranges like `"1.7"`

2. **Code Quality**:
   - Zero compiler errors
   - Zero critical compiler warnings (minor lints acceptable)
   - All examples must compile and run
   - Follow Rust idioms and best practices

3. **Documentation Sync**:
   - Every code change must update corresponding README.md
   - All Cargo.toml snippets must match actual files
   - All code examples must match actual binaries

4. **Testing**:
   - Run `cargo build --bins` before committing
   - Verify examples produce expected output
   - Test on actual machines, not just CI

5. **Commit Messages**:
   - Be specific: "Fix compiler warning in chapter 1 monitor.rs" not "fix stuff"
   - Reference files changed: "Update chapter_2/Cargo.toml"
   - Explain why, not just what: "Remove unnecessary mut from subscriber"

### Common Tasks for Agents

#### Task: Fix a Compiler Warning

Example: Unused variable warning in chapter 2
```bash
# 1. Identify the warning
cargo build --bins 2>&1 | grep "warning:"

# 2. Review the offending code
view examples/tutorials/smart_building/chapter_2/src/bin/floor_monitor.rs

# 3. Make minimal change (remove mut, don't refactor)
edit <file> <old_str> <new_str>

# 4. Verify the fix
cargo build --bins 2>&1 | grep -E "error|warning"

# 5. Commit with specific message
git add <files> && git commit -m "Fix unused variable warning in chapter_2 floor_monitor"
```

#### Task: Update Documentation Example

Example: Cargo.toml snippet is out of date
```bash
# 1. Check actual Cargo.toml
view examples/tutorials/smart_building/chapter_1/Cargo.toml

# 2. Find documentation reference
grep -r "zenoh = {" examples/tutorials/smart_building/chapter_1/README.md

# 3. Update README to match reality
edit examples/tutorials/smart_building/chapter_1/README.md <old> <new>

# 4. Verify no broken links
# Check if README links to other chapters/files

# 5. Commit change
git add examples/tutorials/smart_building/chapter_1/README.md
git commit -m "Update chapter 1 README - Cargo.toml example to use Zenoh 1.7.2"
```

#### Task: Verify All Examples Compile

```bash
# Build all chapters
for i in 1 2 3 4 5; do
  echo "Building chapter_$i..."
  cd examples/tutorials/smart_building/chapter_$i
  cargo build --bins 2>&1 | tail -1
  cd - > /dev/null
done
```

#### Task: Check Documentation Links

```bash
# Find all markdown links
grep -r "\[.*\](.*)" examples/tutorials/ --include="*.md"

# Verify they point to real files
# Use tools/scripts to validate all paths
```

### When to Ask for Human Help

Agents should escalate to humans for:

1. **Architecture decisions**: "Should chapter 6 cover X or Y topic?"
2. **Domain knowledge**: Questions about robotics, IoT, finance, manufacturing
3. **Design reviews**: Major refactoring affecting tutorial flow
4. **API changes**: When Zenoh API changes between versions
5. **Security concerns**: If changes might introduce vulnerabilities

## Testing Guidelines

### Build Verification

Every code change must pass:
```bash
cargo build --bins              # Must succeed with no errors
cargo build --bins 2>&1 | grep "error"  # Should output nothing
```

### Runtime Verification

For changes affecting example behavior:
```bash
# Terminal 1
cargo run --bin sensor

# Terminal 2 (in another shell)
cargo run --bin monitor

# Verify: monitor should receive sensor data
```

### Documentation Verification

For documentation changes:
- All code snippets must match actual source files
- All links must point to real files
- No broken references

## Quality Metrics

The tutorial should maintain:

- ✅ **0 compiler errors** across all chapters
- ✅ **0 critical warnings** (unused imports, missing error handling)
- ✅ **100% link integrity** (no broken internal or external links)
- ✅ **100% documentation-code sync** (README matches actual code)
- ✅ **All examples runnable** (compile and execute successfully)
- ✅ **Consistent Zenoh version** (all chapters use 1.7.2)

## Problem Areas

Known issues to watch for:

1. **Path Dependencies**: Chapters 1-9 Cargo.toml files sometimes revert to local path deps. Always ensure `version = "1.7.2"`

2. **Documentation Drift**: README.md examples may diverge from actual source code
   - Example: Cargo.toml snippets showing old dependencies
   - Solution: Regularly sync README with actual files

3. **Compiler Warnings**: New Rust versions or Zenoh updates may introduce warnings
   - Solution: Run full build regularly and fix issues promptly

4. **Broken Links**: New files added may break existing references
   - Solution: Validate all links after major changes

5. **Platform Differences**: Examples may behave differently on Linux/macOS/Windows
   - Solution: Test on multiple platforms when possible

## Branch and Merge Strategy

### Branch Naming

- Feature branches: `feature/add-chapter-6`
- Bug fixes: `bugfix/fix-compiler-warning`
- Documentation: `docs/update-readme`
- Domain guides: `domain/expand-robotics-guide`

### Commit Strategy

Keep commits small and atomic:
```
Good:   "Fix unused mut in chapter 1 monitor.rs"
        "Update chapter 1 README Cargo.toml example"
        
Avoid:  "Fix various issues in tutorial"
        "Update everything"
```

### PR/MR Checklist

Before merging:
- [ ] All code compiles without errors
- [ ] All compiler warnings resolved or documented
- [ ] Documentation updated to match code
- [ ] Links verified
- [ ] Examples tested and working
- [ ] Commit messages clear and specific

## Useful Commands for Agents

```bash
# Find all Rust files in tutorial
find examples/tutorials -name "*.rs"

# Find all Cargo.toml files
find examples/tutorials -name "Cargo.toml"

# Find all markdown documentation
find examples/tutorials -name "*.md"

# Check for specific patterns (e.g., path dependencies)
grep -r "path = \"" examples/tutorials

# Verify Zenoh version consistency
grep -r "zenoh.*version" examples/tutorials

# Build all chapters with verbose output
for i in 1 2 3 4 5; do
  cargo build --bins -C examples/tutorials/smart_building/chapter_$i
done
```

## Resources

- **Tutorial Entry**: `examples/tutorials/START_HERE.md`
- **Main Index**: `examples/tutorials/INDEX.md`
- **Domain Guides**: `examples/tutorials/DOMAIN_ADAPTATIONS.md`
- **Link Report**: `examples/tutorials/LINK_VERIFICATION_REPORT.md`
- **Zenoh Docs**: https://zenoh.io/docs/
- **Repository**: Current zenoh/zenoh GitHub repository

## Success Criteria for Agent Work

### Code Improvements
- Code compiles without errors ✓
- Compiler warnings addressed ✓
- Code follows Rust idioms ✓
- Changes are minimal and focused ✓

### Documentation Improvements
- Documentation matches actual code ✓
- All links are valid ✓
- Examples are accurate and tested ✓
- No typos or unclear sections ✓

### Testing Improvements
- Tests are comprehensive ✓
- Edge cases covered ✓
- Examples runnable ✓
- Cross-platform compatible ✓

---

**Last Updated**: January 13, 2026
**Tutorial Status**: Phase 5 Complete - Production Ready
**Zenoh Version**: 1.7.2
**Rust Version**: 1.75+
