# Core QUIC Plugins

Simple repository containing some plugins running with Core QUIC.
This is a work in progress!

## Implemented plugins

All the plugins are listed in alphabetical order.

### Functional


### Under development
* `max-data`: simply rewrite processing of max-data

## Compiling plugins

When you want to compile a plugin, go to the root of the related plugin.
Then, you can compile it with the following commands.
```bash
cargo build --target wasm32-unknown-unknown --release
wasm-gc target/wasm32-unknown-unknown/release/plugin_name.wasm plugin_name.wasm
```

## Creating your own plugin

Each plugin has its own crate/project.
To start a new one, at the root of this repository launch this command:
```bash
cargo init --lib plugin-name
```

Then you need to add the following lines to the generated `Cargo.toml` file.
```toml
[dependencies]
pluginop-wasm = { path = "relative/path/to/pluginop-wasm" }

# Indicate that we need to generate a WASM file, otherwise not WASM would be generated at compilation.
[lib]
crate-type =["cdylib"]
```

Note that at some point, `pluginop-wasm` will be published on `crates.io`, and adding this dependency would be done simply using, e.g., `pluginop-wasm = "1"`.

## Quic(k) FAQ

### When I compile, I don't have any WASM file generated.

Seems you forgot to add the following in the `Cargo.toml`.
```toml
# Indicate that we need to generate a WASM file, otherwise not WASM would be generated at compilation.
[lib]
crate-type =["cdylib"]
```

### The name of my protocol operation function is correct, but it is never called.

Two common errors: you forgot either the `pub extern` or the `#[no_mangle]`.
Remember that an exposed protocol operation always looks like the following.
```rust
#[no_mangle]
pub extern fn protocol_operation_name(penv: &mut PluginEnv) -> i64 { /* ... */ }
```