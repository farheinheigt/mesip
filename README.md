# mesip

## Overview
Display local/public/VPN IP information.

## Location
- Repository: `/Users/farheinheigt/Projets/network/mesip`
- User entrypoint: `/Users/farheinheigt/Projets/network/mesip/bin/mesip`
- Completion file: `/Users/farheinheigt/Projets/network/mesip/bin/_mesip.completion.zsh`

## Usage
Run the command directly: `mesip`.
Generate completion script: `mesip --completion zsh`.

## Examples
`mesip`
`mesip --no-public --no-color`
`mesip --timeout 5`
`mesip --completion zsh`

## Requirements
- Runtime wrapper: `zsh`
- Build tool: `cargo`

## Notes
- The user-facing entrypoint remains `bin/mesip`.
- The Rust source lives under `src/`.
- Public IP lookups are done natively from Rust.
- The Cargo build output stays in the repository-local `target/` directory.
