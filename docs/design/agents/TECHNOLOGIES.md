# Technologies to Use

## Formatters

Use the following formatters for the appropriate language types, taking their individual configurations into account:

- Prettier
- rustfmt
- Svelte/Sveltekit

## Other Constraints

- Unless instructed otherwise all Rust must include `unsafe_code = "forbid"` in the `Cargo.toml` for workspaces and packages. Packages should inherit it from the workspace.
- The folder `.github/FILE_TEMPLATES/` contains a number of example configuration files to use as templates when creating new projects or configurations.
- Similarly, there is a `.github/FILE_TEMPLATES/.gitignore.base` to use as a baseline for creating `.gitignore` files.
