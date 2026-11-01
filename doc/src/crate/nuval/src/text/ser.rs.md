# ser.rs

## fn to_nuval
serde's data model maps onto NuVal as serde_json maps it onto JSON: unit and
none are nothing, a newtype is its inner value, a unit variant its name, and
any other variant a one-key record, so the text reads as hand-written NUON.

## fn to_nuon
A NuVal's own text is `NuVal::to_nuon`; serializing a NuVal through serde
writes its derived, tagged form instead.

## struct Entries
A map's keys are unique, since nushell's records refuse a repeated column.

## struct KeySerializer
A map's key becomes a record's string key: strings, chars, numbers, bools,
and unit variants, as serde_json takes them. Numbers read back through the
deserializer's `Key`.
