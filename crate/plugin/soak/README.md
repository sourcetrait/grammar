# Soak plugin for Nushell

## Usage

### soak str
```nu
{ name: World } | soak str 'Hello, {{name}}!'
# Hello, World!
```

### str soak
```nu
'Hello, {{name}}!' | str soak { name: World }
# Hello, World!
```

### soak dir
*See [testing/soak_dir](./testing/soak_dir) and [testing/soaked_dir](./testing/soaked_dir) for example files.*
```nu
{ name: World, color: salmon, impartial: REDACTED } | soak dir ./testing/soak_dir /tmp/soaked_dir
```
