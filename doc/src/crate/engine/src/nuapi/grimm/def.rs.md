# def.rs

The `grimm` commands' words as data: each command's `SignatureDef`, its name,
description, category, and examples, and its parameters' `ParameterDef`s, from which
`signature()` builds the nushell signature through nuin's `vocab` builders. One
definition feeds the signature, `help`, and the examples alike, so they cannot
disagree about a command.

## impl GrimmChannelSend

### const DEF
Two examples of one send: the value the definition gets back, `{event_id}`, and the
packet the Channel delivers for it, so `help` shows both halves of a send.
