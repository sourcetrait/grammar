# de.rs

## impl de::Deserializer for NuVal
`deserialize_any` gives each value serde's nearest form: a filesize and a
duration their ints, a date, a range, and a cell path their raw NUON text,
which chrono's own deserializer reads for a date. An error takes each record
field and list index as it unwinds.

## struct Key
A record's keys are strings; a map keyed by numbers or bools reads them back
by parsing each key.
