# Claude Code Instructions for rust-claude-codes

## Code Quality Standards

**IMPORTANT: Before every commit, you MUST:**
1. Run `cargo fmt --all` to format all Rust code
2. Run `cargo clippy --all-targets --all-features -- -D warnings` and fix all warnings  
3. Ensure all tests pass with `cargo test --all`

**CRITICAL: ALWAYS run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings` before EVERY commit without exception. This is non-negotiable.**

**NOTE: CI will fail if there are clippy warnings or formatting issues, so please fix them before committing.**

## Git Commit Guidelines

**CRITICAL**: NEVER use `git add -A` or `git add .` when committing changes!
- Always use `git add -u` to stage only modified files, or
- Use `git add <specific-file>` to stage specific files
- The `-A` flag can accidentally add unintended files and cause issues
- Review `git status` before committing to ensure no unwanted files are staged

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