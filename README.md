# mesip

Display local, public, and VPN IP information from a small autonomous Rust CLI.

## Entrypoints

- User command: `bin/mesip`
- Zsh completion: `bin/_mesip.completion.zsh`
- Rust source: `src/main.rs`

## Usage

Run the command directly: `bin/mesip`.
Generate completion script: `bin/mesip --completion zsh`.

## Examples

`bin/mesip`
`bin/mesip --no-public --no-color`
`bin/mesip --timeout 5`
`bin/mesip --completion zsh`

## Requirements

- Runtime wrapper: `zsh`
- Build tool: `cargo`

## Notes

- The user-facing entrypoint remains `bin/mesip`.
- The Rust source lives under `src/`.
- Public IP lookups are done natively from Rust.
- The Cargo build output stays in the repository-local `target/` directory.
