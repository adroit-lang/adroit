# Adroit

An experimental differentiable programming language.
[(GitHub)](https://github.com/adroit-lang/adroit)

## Installation

The easiest way to install Adroit is to download the prebuilt binary for your
platform from the latest GitHub release:

### macOS Apple Silicon

```sh
sudo curl -L https://github.com/adroit-lang/adroit/releases/download/v0.2.2/adroit-aarch64-apple-darwin -o /usr/local/bin/adroit
sudo chmod +x /usr/local/bin/adroit
```

### macOS Intel

```sh
sudo curl -L https://github.com/adroit-lang/adroit/releases/download/v0.2.2/adroit-x86_64-apple-darwin -o /usr/local/bin/adroit
sudo chmod +x /usr/local/bin/adroit
```

### Linux x64

```sh
sudo curl -L https://github.com/adroit-lang/adroit/releases/download/v0.2.2/adroit-x86_64-unknown-linux-musl -o /usr/local/bin/adroit
sudo chmod +x /usr/local/bin/adroit
```

### Building from source

If none of those prebuilt binaries work for you, you can instead build Adroit
from source. First install [Git][] and [Rust][], then run these commands:

```sh
git clone --depth 1 --branch v0.2.2 https://github.com/adroit-lang/adroit.git
cd adroit
cargo install --locked --path crates/adroit
```

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

By convention, Adroit source file names end with the `.adroit` extension.

## Language

Here is an Adroit function that takes a floating-point number as input, and
returns the square of that number as output:

```adroit
def square(x: Float): Float = x * x
```

Within the body of a function, you can use `let` to give an intermediate value a
name; then you write the return value of the function after all your
`let`-bindings:

```adroit
def foo(x: Float, y: Float): Float =
  let a = x + y
  let b = x * y
  a / b
```

Indentation is never significant in Adroit, but newlines are sometimes
significant; as you can see above, you can end a `let`-binding with a newline,
but if you'd prefer to put more on one line, you can instead use a semicolon:

```adroit
def tesseract(x: Float): Float = let y = x * x; y * y
```

Functions can be generic:

```adroit
def identity[T](x: T): T = x
```

Adroit currently has three standard library modules:

- `"array"`
- `"autodiff"`
- `"math"`

Here is an example using a couple functions from the `"array"` module to
implement matrix multiplication:

```adroit
import "array" use for, sum

def mmul[M, N, P](
  a: [M * N]Float,
  b: [N * P]Float
): [M * P]Float =
  for (i, j) =>
    sum(for k => a[i, k] * b[k, j])
```

[from the VS Code Marketplace]: https://marketplace.visualstudio.com/items?itemName=adroit-lang.adroit-vscode
[git]: https://git-scm.com/downloads
[rust]: https://www.rust-lang.org/tools/install
