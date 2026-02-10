# GitHub Repository Setup

## Push to Querent-ai Organization

This repository is configured for: **https://github.com/Querent-ai/openvds-rs**

### Initial Setup

```bash
cd /home/querent/oilAndgas/openvds-rs

# Initialize git (if not already done)
git init

# Add all files
git add .

# Create initial commit
git commit -m "Initial commit: openvds-rs v0.2.0

- Pure Rust OpenVDS format library
- Local filesystem I/O with async support
- Extensible IOManager trait for cloud storage
- Comprehensive CI pipeline (GitHub Actions)
- Complete documentation and examples
- Apache 2.0 license
- Test data from OpenVDS repository"

# Add remote (create repo on GitHub first!)
git remote add origin https://github.com/Querent-ai/openvds-rs.git

# Push to GitHub
git branch -M main
git push -u origin main
```

### Before Pushing - Create GitHub Repository

1. Go to https://github.com/Querent-ai
2. Click "New repository"
3. Repository name: `openvds-rs`
4. Description: `Pure Rust OpenVDS implementation - async I/O for multi-dimensional volumetric data`
5. Visibility: Public (or Private if preferred)
6. **DO NOT** initialize with README, .gitignore, or license (we have these locally)
7. Click "Create repository"
8. Then run the git commands above

### Repository Settings (After Creation)

#### About Section
- Description: `Pure Rust OpenVDS implementation - async I/O for multi-dimensional volumetric data`
- Website: Leave empty or add docs link
- Topics/Tags: `rust`, `openvds`, `seismic`, `volumetric-data`, `async`, `geophysics`

#### GitHub Actions
- Actions should automatically run on push
- All CI jobs should pass (build, test, clippy, docs, etc.)

#### Branch Protection (Recommended)
For `main` branch:
- ✅ Require a pull request before merging
- ✅ Require status checks to pass before merging
  - Select: CI / fmt, CI / clippy, CI / test, CI / docs
- ✅ Require branches to be up to date before merging

### Repository Structure

```
https://github.com/Querent-ai/openvds-rs/
├── .github/
│   └── workflows/
│       └── ci.yml           # CI pipeline
├── examples/
│   ├── seismic_volume.rs
│   ├── concurrent_loading.rs
│   └── custom_io_backend.md
├── src/
│   └── [Rust source code]
├── test-data/               # .gitignored
├── ARCHITECTURE.md
├── CHANGELOG.md
├── CLOUD_STORAGE.md
├── Cargo.toml
├── LICENSE
├── Makefile
├── QUICKSTART.md
├── README.md
├── STATUS.md
└── rust-toolchain.toml
```

### CI Status Badge

After first push, add this to README.md (top):

```markdown
[![CI](https://github.com/Querent-ai/openvds-rs/workflows/CI/badge.svg)](https://github.com/Querent-ai/openvds-rs/actions)
```

### Releases

When ready for first release:

```bash
# Tag the release
git tag -a v0.2.0 -m "Release v0.2.0: Initial public release

- Pure OpenVDS format library (read-only)
- Local filesystem support
- IOManager trait for cloud storage extensions
- Comprehensive CI and documentation"

# Push tags
git push origin v0.2.0
```

Then create GitHub Release:
1. Go to Releases → Draft a new release
2. Choose tag: v0.2.0
3. Release title: `v0.2.0 - Initial Release`
4. Description: Copy from CHANGELOG.md
5. Attach: None needed (source code auto-attached)
6. Publish release

### Publishing to crates.io (Future)

When ready to publish:

1. Ensure Cargo.toml is correct
2. Login: `cargo login`
3. Publish: `cargo publish`

Note: This will make the crate available as:
```toml
[dependencies]
openvds = "0.2"
```

### Maintenance

#### Update Dependencies
```bash
cargo update
cargo test
git commit -am "chore: update dependencies"
```

#### After Changes
```bash
# Always run local CI first
make ci-local

# If passing, commit and push
git add .
git commit -m "your message"
git push
```

#### Monitor CI
- Check https://github.com/Querent-ai/openvds-rs/actions
- All jobs should pass (green ✅)
- Fix any failures before merging PRs

### Team Access

Organization owners can add contributors:
1. Settings → Manage access
2. Invite a collaborator
3. Choose role: Write (most contributors) or Maintain (project leads)

### Links

- Repository: https://github.com/Querent-ai/openvds-rs
- Issues: https://github.com/Querent-ai/openvds-rs/issues
- Actions: https://github.com/Querent-ai/openvds-rs/actions
- Releases: https://github.com/Querent-ai/openvds-rs/releases

## Quick Commands

```bash
# Check status
make ci-local

# Format and lint
make fmt lint

# Build and test
make build test

# Run examples
make examples

# Clean everything
make clean-all
```

## Notes

- Test data (test-data/) is gitignored (run `make download-test-data` locally)
- Cargo.lock is gitignored (library project)
- CI runs on every push to main and all PRs
- One pre-existing test failure in layout::tests::test_brick_index_conversion
