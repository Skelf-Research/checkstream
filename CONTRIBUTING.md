# Contributing to CheckStream

Thank you for your interest in contributing to CheckStream! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- **Rust**: Version 1.75 or higher
- **Cargo**: Latest stable version
- **Git**: For version control

### Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/Skelf-Research/checkstream.git
cd checkstream

# Build the project
cargo build

# Run tests
cargo test

# Run the proxy (development)
cargo run --bin checkstream-proxy -- --help
```

## Project Structure

CheckStream is organized as a Cargo workspace with multiple crates:

- **checkstream-core**: Core types, traits, and utilities
- **checkstream-proxy**: HTTP/SSE proxy implementation
- **checkstream-policy**: Policy engine and rule evaluation
- **checkstream-classifiers**: Safety and compliance classifiers
- **checkstream-telemetry**: Metrics and audit trail functionality

## Development Guidelines

### Code Style

- Follow standard Rust formatting: `cargo fmt`
- Run Clippy for linting: `cargo clippy --all-targets --all-features`
- Ensure all tests pass: `cargo test --all`

### Performance Considerations

CheckStream is designed for **high performance** with strict latency budgets:

- Target sub-10ms overhead for proxy operations
- Optimize hot paths and minimize allocations
- Use zero-copy operations where possible
- Profile with `cargo bench` before and after changes

### Testing

- Write unit tests for new functionality
- Add integration tests for end-to-end scenarios
- Include benchmarks for performance-critical code
- Test with `cargo test --all-features`

### Documentation

- Document all public APIs with rustdoc comments
- Include examples in documentation
- Update relevant markdown docs in `/docs`
- Run `cargo doc --open` to preview documentation

## Pull Request Process

1. **Fork the repository** and create a feature branch
2. **Make your changes** following the guidelines above
3. **Write tests** for new functionality
4. **Update documentation** as needed
5. **Run all checks**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features
   cargo test --all
   cargo bench  # if you changed performance-critical code
   ```
6. **Submit a pull request** with a clear description of changes

### Pull Request Template

When submitting a PR, include:

- **Description**: What does this PR do?
- **Motivation**: Why is this change needed?
- **Testing**: How was this tested?
- **Performance**: Impact on latency/throughput (if applicable)
- **Breaking Changes**: Any API changes or compatibility issues?

## Types of Contributions

### Bug Fixes

- Search existing issues before creating a new one
- Provide a minimal reproduction case
- Include relevant logs or error messages

### New Features

- Open an issue first to discuss the feature
- Ensure it aligns with CheckStream's goals
- Consider performance implications
- Document the feature thoroughly

### Classifier Implementations

If contributing a new classifier:

- Place it in `crates/checkstream-classifiers/src/`
- Implement the `Classifier` trait
- Specify the appropriate tier (A/B/C) based on latency
- Include benchmarks demonstrating sub-budget performance
- Add tests with various input scenarios

### Policy Packs

For new policy packs:

- Create YAML files in `/policies`
- Map rules to specific regulations
- Include clear descriptions and regulation references
- Add examples and test cases

## Performance Targets

When contributing code, ensure it meets these targets:

| Component | Target Latency |
|-----------|---------------|
| Tier A Classifiers | <2ms |
| Tier B Classifiers | <5ms |
| Tier C Classifiers | <10ms |
| Policy Evaluation | <1ms |
| Proxy Overhead (total) | <10ms |

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the code, not the person
- Help create a welcoming environment for all contributors

## Security

For security vulnerabilities:

- **DO NOT** open a public issue
- Email security@checkstream.ai with details
- Allow time for patches before public disclosure

## Regulatory Compliance

When working on compliance features:

- Cite specific regulation sections (e.g., "FCA PRIN 2A.4.1")
- Ensure audit trail completeness
- Test with representative scenarios
- Document regulatory mappings clearly

## Questions?

- Open a GitHub issue for general questions
- Check existing documentation in `/docs`
- Join discussions in pull requests

## License

By contributing to CheckStream, you agree that your contributions will be licensed under the Apache License 2.0.

---

Thank you for contributing to CheckStream!
