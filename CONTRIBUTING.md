# Contributing to chd2iso-fuse

Thank you for your interest in contributing!

## Ways to contribute

* 🐛 **Report bugs** via GitHub Issues
* 💡 **Suggest features** or improvements
* 🔧 **Submit pull requests** for fixes or enhancements
* 📝 **Improve documentation**

## Pull Request Guidelines

* **Target the `develop` branch.** All new pull requests should be opened against `develop`. The `main` branch is reserved for stable releases.
* Keep changes focused — avoid unrelated modifications.
* Follow Rust formatting standards by running `cargo fmt`.
* Run `cargo clippy` to catch common mistakes.
* Ensure the project builds successfully in release mode.
* If your change affects behaviour, please include or update tests where appropriate.
* Update documentation if your changes introduce new features, options or user-visible behaviour.

## Setting up a development environment

```bash
git clone https://github.com/lloydsmart/chd2iso-fuse.git
cd chd2iso-fuse
git checkout develop
cargo build
```

## Before submitting a pull request

Please run the following before opening your PR:

```bash
cargo fmt
cargo clippy
cargo test
cargo build --release
```

Thank you for helping improve chd2iso-fuse!
