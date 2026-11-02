Runs an embedded Grammar engine command directly.

Runs a single embedded engine `command`'s `subcommand` in an isolated Nu
context without access to any other commands.

## Example
Tool parameter `command`:
```nu
grmr
```
Tool parameter `subcommand`:
```nu
channel send 'my/adhoc/Thing' {foo: 'bar', num: 3}
```
Tool result:
```nuon
{nonce: "gh21YxiwXAQ", result: {event_id: "5zNe1n9JO9G"}}
```
