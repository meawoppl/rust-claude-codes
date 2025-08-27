# Integration Tests

This directory contains integration tests that interact with real Claude CLI services.

## Running Integration Tests

Integration tests are behind a feature flag and require a working Claude CLI installation.

### Prerequisites

1. Install Claude CLI: `npm install -g @anthropic-ai/claude-cli`
2. Authenticate: `claude login`
3. Verify installation: `claude --version`

### Running Tests

```bash
# Run all integration tests
cargo test --features integration-tests

# Run with output
cargo test --features integration-tests -- --nocapture

# Run a specific test
cargo test --features integration-tests test_async_client_basic_query
```

### Test Coverage

The integration tests cover:

- **Version Check**: Verifies Claude CLI is installed and accessible
- **Basic Queries**: Tests simple question/answer interactions with both async and sync clients
- **Conversations**: Tests multi-turn conversations with context retention
- **Tool Use**: Tests Claude's ability to use tools (when available)
- **Error Handling**: Tests behavior with invalid configurations

### CI/CD Note

Integration tests are NOT run in CI by default since they require:
- Claude CLI installation
- Valid authentication
- Network access to Claude services

To run them locally, use the feature flag as shown above.

### Writing New Integration Tests

When adding new integration tests:

1. Always use the `#[cfg(feature = "integration-tests")]` attribute
2. Use reasonable timeouts (30 seconds max)
3. Clean up resources (sessions) when possible
4. Document what the test is validating
5. Handle potential network failures gracefully