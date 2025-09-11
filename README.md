# Lunaris - a truly stellar video editor.

## This repo is in rapid development.

Do not assume that anything, including the API is stable.

# How to build

## Just run(debug)

`just run`

## Just run(release)

`just rr`

## Build

`just build-full`

## Plugins

Plugins are statically linked and auto-discovered via `inventory`.

- In a plugin crate: `cargo add lunaris_api inventory`, implement `Plugin` (and optional `Gui`).
- Register with a single line, providing a name literal:
  - `lunaris_api::register_plugin!(MyPlugin, name: "My Plugin");`
  - or `lunaris_api::register_plugin_gui!(MyPlugin, name: "My Plugin");`
- The host discovers all linked plugins at startup; no `prepare/cleanup` step is required.
