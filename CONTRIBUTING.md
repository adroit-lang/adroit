# Contributing to Adroit

## Prerequisites

Make sure to have these tools installed:

- [Git][]
- [Rust][]
- [Node][]

Other tools that are optional but useful:

- [GitHub CLI][]

## Setup

Once you've installed all prerequisites, clone this repo, e.g. with GitHub CLI:

```sh
gh repo clone adroit-lang/adroit
```

Then open a terminal in your clone of it; for instance, if you cloned it via the
terminal, run this command:

```sh
cd adroit
```

## Rust

Run this command to build and test the main Rust codebase:

```sh
cargo test
```

## Node

We use Node.js for our VS Code extension. To work with the Node packages in this
repository, first install all dependencies from npm:

```sh
npm install
```

### VS Code

On Linux or macOS, run these commands to build the VS Code extension (on
Windows, the binary filename will end with `.exe`):

```sh
cargo build --release
cp target/release/adroit packages/vscode/bin/adroit
npm run --workspace=adroit-vscode build
```

Then in the VS Code Explorer, right-click on the
`packages/vscode/adroit-vscode-*.vsix` file that has been created, and click
**Install Extension VSIX**.

[git]: https://git-scm.com/downloads
[github cli]: https://github.com/cli/cli#installation
[node]: https://nodejs.org/en/download
[rust]: https://www.rust-lang.org/tools/install
