# Contributing to Tangle Network Gadget

Thank you for your interest in contributing to the Tangle Network Gadget project! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Getting Started](#getting-started)
  - [Development Environment](#development-environment)
  - [Project Structure](#project-structure)
- [Making Contributions](#making-contributions)
  - [Pull Request Process](#pull-request-process)
  - [Commit Messages](#commit-messages)
  - [Branch Naming](#branch-naming)
- [Development Guidelines](#development-guidelines)
  - [Code Style](#code-style)
  - [Testing Requirements](#testing-requirements)
  - [Documentation](#documentation)
- [Review Process](#review-process)

## Getting Started

### Development Environment

1. Install required dependencies:
   ```bash
   # Ubuntu/Debian
   apt install build-essential cmake libssl-dev pkg-config

   # macOS
   brew install openssl cmake
   ```

2. Set up Rust:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install specific toolchain
   rustup install nightly-2024-10-13
   rustup default nightly-2024-10-13
   
   # Install required components
   rustup component add rustfmt clippy rust-src
   ```

3. Clone the repository:
   ```bash
   git clone https://github.com/tangle-network/gadget.git
   cd gadget
   ```

### Project Structure

Please familiarize yourself with the project structure before contributing:

- `cli/`: Command-line interface implementation
- `crates/`: Core functionality modules
- `.github/`: GitHub-specific files (workflows, templates)

## Making Contributions

### Pull Request Process

1. Fork the repository and create your feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes, following our [development guidelines](#development-guidelines)

3. Run the test suite:
   ```bash
   cargo test
   ```

4. Update documentation as needed

5. Submit a pull request with a clear description of the changes and any relevant issue numbers

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, missing semi-colons, etc)
- `refactor`: Code refactoring
- `test`: Adding missing tests
- `chore`: Maintenance tasks

Example:
```
feat(cli): add new blueprint deployment option

Added --dry-run flag to deployment command for testing deployments
without actually submitting transactions.

Closes #123
```

### Branch Naming

Use the following naming convention for branches:
- `feature/`: New features
- `fix/`: Bug fixes
- `docs/`: Documentation updates
- `refactor/`: Code refactoring
- `test/`: Test-related changes

Example: `feature/blueprint-deployment`

## Development Guidelines

### Code Style

1. Follow Rust style guidelines
2. Use `rustfmt` for code formatting:
   ```bash
   cargo fmt
   ```
3. Run clippy for linting:
   ```bash
   cargo clippy -- -D warnings
   ```

### Testing Requirements

1. Write unit tests for new functionality
2. Ensure existing tests pass
3. Add integration tests for new features
4. Include documentation tests for public APIs
5. Use async/tokio for asynchronous tests

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_feature() {
        // Async test implementation
        let result = some_async_function().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Async error handling test
        match failing_async_function().await {
            Err(e) => assert_matches!(e, Error::Expected),
            _ => panic!("Expected error"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_concurrent_operations() {
        // Test concurrent operations
        let (result1, result2) = tokio::join!(
            async_operation1(),
            async_operation2()
        );
        assert!(result1.is_ok() && result2.is_ok());
    }
}
```

For mocking time-dependent tests:
```rust
use tokio::time::{self, Duration};

#[tokio::test]
async fn test_with_time() {
    let mut interval = time::interval(Duration::from_secs(1));
    
    // First tick completes immediately
    interval.tick().await;
    
    // Use time::sleep for testing timeouts
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Test after delay
    assert!(some_condition);
}
```

### Documentation

1. Document all public APIs
2. Include examples in documentation
3. Update README.md if needed
4. Add inline comments for complex logic

Example documentation:
```rust
/// Deploys a blueprint to the network
///
/// # Arguments
///
/// * `name` - Name of the blueprint
/// * `config` - Blueprint configuration
///
/// # Returns
///
/// * `Result<DeploymentId>` - Deployment identifier on success
///
/// # Examples
///
/// ```
/// let result = deploy_blueprint("my_blueprint", config)?;
/// ```
pub fn deploy_blueprint(name: &str, config: Config) -> Result<DeploymentId> {
    // Implementation
}
```

## Review Process

1. All PRs require at least one review from a maintainer
2. CI checks must pass
3. Documentation must be updated
4. Changes should be tested on a development network
5. Large changes may require multiple reviews

For questions or clarifications, please open an issue or join our [Discord server](https://discord.com/invite/cv8EfJu3Tn).
