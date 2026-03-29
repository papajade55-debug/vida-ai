# Contributing to Vida AI

Thank you for your interest in contributing!

## Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Run tests: `cargo test --workspace && npm run lint`
5. Commit with a descriptive message
6. Open a Pull Request

## Code Style

- **Rust**: Follow `rustfmt` defaults. Use `thiserror` for errors. Tests in `#[cfg(test)]` modules.
- **TypeScript**: Strict mode. Functional components. Named exports.
- **CSS**: Use CSS custom properties from `design-system/tokens.css`. No hardcoded colors.

## Translations

Add locale files to `src/locales/{lang-code}/common.json`. Copy the structure from `src/locales/en/common.json`.

## Adding a Provider

1. Create `crates/vida-providers/src/your_provider.rs`
2. Implement the `LLMProvider` trait
3. Add `pub mod your_provider;` to `lib.rs`
4. Add tests

## Reporting Issues

Use GitHub Issues. Include: OS, Vida AI version, steps to reproduce, expected vs actual behavior.
