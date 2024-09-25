# Contributing to Adroit

## Prerequisites

Make sure to have these tools installed:

- [Git][]
- [Rust][]
- [Bun][]

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

Run this command to make a release build and install it on your `PATH`:

```sh
cargo install --locked --path crates/adroit
```

## JavaScript

We use Bun for our [website][] and for our VS Code extension. To work with the
JavaScript packages in this repository, first install all dependencies from npm:

```sh
bun install
```

### Website

To develop the website locally, run this command:

```sh
bun run --filter=@adroit-lang/website watch
```

Then in a separate terminal:

```sh
bun run --filter=@adroit-lang/website serve
```

### VS Code

On Linux or macOS, run these commands to build the VS Code extension (on
Windows, the binary filename will end with `.exe`):

```sh
cargo build --release
mkdir packages/adroit-vscode/bin
cp target/release/adroit packages/adroit-vscode/bin/adroit
bun run --filter=adroit-vscode build
```

Then in the VS Code Explorer, right-click on the
`packages/adroit-vscode/adroit-vscode-*.vsix` file that has been created, and
click **Install Extension VSIX**.

[bun]: https://bun.sh/
[git]: https://git-scm.com/downloads
[github cli]: https://github.com/cli/cli#installation
[rust]: https://www.rust-lang.org/tools/install
[website]: https://adroit-lang.org
