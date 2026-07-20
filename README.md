# Mast

Mast is a beautiful, free and open-source wiki engine written in Rust.

- Hypersonic speed
- No Database
- No runtime dependencies
- Runs on a potato
- Super configurable
- Easiest install & usage
- Powerful access control
- Extensible with plugins
- Free and open-source, forever

Mast has been under development since mid-2026, and so its localisation/security/plugin ecosystem may not be as mature as other wiki engines.

## Development
Run
```bash
MAST_DEV=1 cargo run
```

in the home directory to start the dev server. You need to have `cargo` installed. The default port is `:3000` however this is configurable later through the config.

### Toolchain
Mast's frontend is created with TailwindCSS and DaisyUI. Therefore, Mast requires `node` as a dev dependency during development, but no node is shipped to the end product!


Made with ❤️ by Cieron in 🇳🇿
