# Adroit

An experimental differentiable programming language.
[(GitHub)](https://github.com/adroit-lang/adroit)

## Installation

The easiest way to install Adroit is to download the prebuilt binary for your
platform from the latest GitHub release:

### macOS Apple Silicon

```sh
sudo curl -L https://github.com/adroit-lang/adroit/releases/download/v0.2.2/adroit-aarch64-apple-darwin -o /usr/local/bin/adroit && sudo chmod +x /usr/local/bin/adroit
```

### macOS Intel

```sh
sudo curl -L https://github.com/adroit-lang/adroit/releases/download/v0.2.2/adroit-x86_64-apple-darwin -o /usr/local/bin/adroit && sudo chmod +x /usr/local/bin/adroit
```

### Linux x64

```sh
sudo curl -L https://github.com/adroit-lang/adroit/releases/download/v0.2.2/adroit-x86_64-unknown-linux-musl -o /usr/local/bin/adroit && sudo chmod +x /usr/local/bin/adroit
```

### Building from source

If none of those prebuilt binaries work for you, you can instead build Adroit
from source by following the [`CONTRIBUTING.md`][] instructions in the GitHub
repo.

## Editor support

### VS Code

Install the Adroit extension [from the VS Code Marketplace][] to get syntax
highlighting, inline error messages, and type information on hover.

## Usage

Once you have Adroit installed, you can run it from the command line:

```sh
adroit --help
```

Currently Adroit is in the early stages of development and has no interpreter
and no compiler backend, so there is no way to run an Adroit program.

## Language

Here is an example function implementing matrix multiplication in Adroit:

```adroit
import "array" use for, sum

def mmul[M, N, P](
  a: [M * N]Float,
  b: [N * P]Float
): [M * P]Float =
  for (i, j) =>
    sum(for k => a[i, k] * b[k, j])
```

[`CONTRIBUTING.md`]: https://github.com/adroit-lang/adroit/blob/main/CONTRIBUTING.md
[from the VS Code Marketplace]: https://marketplace.visualstudio.com/items?itemName=adroit-lang.adroit-vscode
