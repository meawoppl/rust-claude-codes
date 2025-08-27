# Claude Code Instructions for rust-claude-codes

## Development Strategy

### Test-Driven Protocol Development

This project follows a test-driven development approach for implementing the Claude Code JSON protocol:

1. **Discover Protocol Through Usage**: Run examples to interact with Claude and discover new message types
2. **Capture Failed Cases**: Any JSON that fails to deserialize is automatically saved to `test_cases/failed_deserializations/`
3. **Format Test Cases**: Run `./format_test_cases.sh` to ensure JSON formatting
4. **Implement Missing Types**: Add the necessary variants to `ClaudeOutput` enum in `src/io.rs`
5. **Verify Implementation**: Run `cargo test deserialization` to ensure the new types deserialize correctly
6. **Lock in Progress**: Successful test cases prove our protocol implementation is correct

## Git Workflow Requirements

**CRITICAL: This repository enforces a strict PR-based workflow**

### Getting Started

```bash
# Clone the repository
git clone https://github.com/meawoppl/rust-claude-codes
cd rust-claude-codes

# Install git hooks (required for all contributors)
./setup_hooks.sh

# Create a feature branch for your work
git checkout -b feature/your-feature-name

# Make your changes, then commit (avoid git add -A!)
git add -u  # Stage modified files only
git commit -m "Your descriptive commit message"

# Push to GitHub and create a PR
git push origin feature/your-feature-name
```

### Branch Protection & Git Hooks

- **NEVER commit directly to main branch**
- All changes MUST go through feature branches and pull requests
- The pre-commit hook will block direct commits to main

### Pre-commit Checks

The repository has git hooks that enforce:
- No commits to main branch
- Code formatting with `cargo fmt`
- All clippy warnings resolved
- All tests passing
- JSON test cases properly formatted

If you haven't already, run `./setup_hooks.sh` to install these hooks.

### CI/CD Requirements

All PRs must pass the following GitHub Actions checks:

- ✅ Code formatting (`cargo fmt --all -- --check`)
- ✅ Clippy linting (`cargo clippy --all-targets --all-features -- -D warnings`)
- ✅ All tests passing (`cargo test --all-features`)
- ✅ JSON test cases properly formatted
- ✅ All examples compile (`./build_examples.sh`)
- ✅ Documentation builds (`cargo doc --no-deps`)
- ✅ MSRV compatibility (Rust 1.85+)

## Protocol Implementation Guidelines

When implementing new message types:

1. **Start with the test case** - Look at the failed JSON in `test_cases/`
2. **Identify the structure** - Note field names, types, and nesting
3. **Add to ClaudeOutput enum** - Create a new variant with appropriate struct
4. **Follow existing patterns** - Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
5. **Test immediately** - Run `cargo test deserialization` to verify
6. **Document the type** - Add doc comments explaining when this message appears

## Code Quality Standards

**IMPORTANT: Before every commit, you MUST:**
1. Run `cargo fmt --all` to format all Rust code
2. Run `cargo clippy --all-targets --all-features -- -D warnings` and fix all warnings  
3. Ensure all tests pass with `cargo test --all`

**CRITICAL: ALWAYS run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings` before EVERY commit without exception. This is non-negotiable.**

**NOTE: CI will fail if there are clippy warnings or formatting issues, so please fix them before committing.**

## Git Commit Guidelines

### ⚠️ ABSOLUTELY FORBIDDEN: NEVER USE `git add -A` ⚠️

**CRITICAL**: The `git add -A` command is STRICTLY PROHIBITED in this repository!

**WHY THIS MATTERS:**
- `git add -A` stages ALL files including untracked files, temp files, build artifacts, and other random crap
- This has repeatedly caused issues with unwanted files being committed
- It can expose sensitive information, break builds, and pollute the repository

**WHAT TO USE INSTEAD:**
- `git add -u` - Stages only modified tracked files (PREFERRED)
- `git add <specific-file>` - Stage specific files by name
- `git add src/` - Stage specific directories if needed
- ALWAYS run `git status` first to review what will be staged

**UNACCEPTABLE:**
```bash
git add -A          # NEVER DO THIS
git add --all       # NEVER DO THIS
git add .           # AVOID THIS TOO
```

**CORRECT:**
```bash
git status          # Review changes first
git add -u          # Stage modified files only
git add src/io.rs   # Or stage specific files
```

Remember: It's better to run `git add` multiple times for specific files than to accidentally commit garbage with `-A`.

## Rust Development Standards

### Code Organization
- Follow Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Use descriptive variable names
- Keep functions focused and under 50 lines when possible
- Document public APIs with rustdoc comments
- Place shared code in appropriate modules to avoid duplication

### Error Handling
- Use Result types for fallible operations
- Provide meaningful error messages
- Implement proper error propagation with `?` operator
- Consider using `thiserror` or `anyhow` for error management

### Testing Requirements
- Write unit tests for all business logic
- Use `#[cfg(test)]` modules for test code
- Mock external dependencies in tests
- Aim for high test coverage
- Use property-based testing with `quickcheck` or `proptest` where applicable

### Performance Considerations
- Profile before optimizing
- Use `&str` instead of `String` when ownership isn't needed
- Prefer iterators over collecting into intermediate vectors
- Use `Cow` for potentially-borrowed data
- Consider using `Arc` or `Rc` for shared ownership when appropriate

### Async Programming
- Use `tokio` for async runtime when needed
- Properly handle async errors
- Avoid blocking operations in async contexts
- Use `tokio::spawn` for concurrent tasks

## Workflow Commands

When I say:
- **"complete"**: Run `cargo fmt --all`, fix clippy issues with `cargo clippy --all-targets --all-features -- -D warnings`, then commit and push
- **"freshen"**: Pull main and merge into the current branch
- **"merge main"**: Pull the remote main branch and merge it into the current working branch

## Development Philosophy

### Incremental Development
- Build from smallest testable pieces
- Validate each component before integration
- Layer functionality incrementally
- Maintain working state at each step

### Code Quality Over Speed
- Don't take shortcuts that compromise quality
- Handle edge cases properly
- Consider error paths thoroughly
- Write code that's maintainable and clear

### Testing First
- Write tests alongside implementation
- Test edge cases and error conditions
- Ensure tests are deterministic and reliable
- Keep tests focused and independent

## Important Reminders
- Don't use esoteric scripts to edit code - use direct read/write operations
- Always run fmt and clippy before committing
- Never create files unless absolutely necessary
- Prefer editing existing files over creating new ones
- Update documentation when making significant changes

## Version Management
When updating the version number in `Cargo.toml`:
1. Update the version field in `Cargo.toml`
2. Run `cargo build` to regenerate `Cargo.lock` with the new version
3. Commit both `Cargo.toml` and `Cargo.lock` together
4. Use a commit message like: "chore: bump version to X.Y.Z"

This ensures the lockfile stays in sync with the version number.