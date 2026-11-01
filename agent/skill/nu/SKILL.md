---
name: nu
description: Additional Nu (Nushell) language documentation
---
# Nu Language
Instruction through example.

## Types & Literals
Nu is strongly + gradually typed: types are enforced, annotations optional.
`describe` reports the runtime type; `into <type>` casts between types.
A list of records IS a table; records carry their field types.
```nu
# describe reports the concrete type
[foo bar] | describe # result: list<string>

# a list of records is a table
[{a: 1}] | describe # result: table<a: int>

# records carry their field types
{a: 1 b: 2} | describe # result: record<a: int, b: int>
```

int: signed 64-bit, two's complement. Literals: decimal, 0x hex, 0o octal,
0b binary; leading + allowed. `_` may separate digits (1_000_000).
```nu
# hex/octal/binary literals are ints
[0xff, 0o17, 0b101] # result: [255 15 5]
```

float: IEEE-754 f64, APPROXIMATE (10.2 * 5.1 is 52.01999999999999).
`number` (a signature shape, not a stored type) accepts int OR float.

duration: number + unit, stored as nanoseconds. Units: ns, us/microsecond,
ms, sec, min, hr, day, wk. Fractional ok (1.5day). No month/year (imprecise).
datetime + duration works; datetime - datetime = duration; dividing two
durations yields a plain number.
```nu
# durations do arithmetic
30day / 1sec # result: 2592000
```

filesize: number + unit. Metric base-1000 (kB MB GB TB PB EB) AND binary
base-1024 (KiB MiB GiB ...). Units are case-insensitive (1kib == 1KiB).
```nu
# filesize supports metric and binary units and math
1GiB / 1B # result: 1073741824

# filesize renders with a unit
0.5kB | into string # result: 500 B
```

range: start..end (INCLUSIVE). start..<end excludes the end. A three-part
range start..second..end sets the stride to (second - start) - the second
value is literal, NOT a step count. Start/end optional (..5 == 0..5; 1.. is
infinite). Ranges are LAZY; pair an open range with `take`. A range does NOT
pipe into list-input commands - `1..5 | length` fails at PARSE (InputMismatch,
"Command does not support range input"); `each`/`first`/`take` DO accept a
range, so the examples below use `each {|x| $x}` to realize one as a list.
```nu
# the middle value is the second element, not the step
1..3..10 | each {|x| $x } # result: [1 3 5 7 9]

# ..< excludes the upper bound
1..<5 | each {|x| $x } # result: [1 2 3 4]

def ranges [] { 1.. | take 5 } # result: [1 2 3 4 5]
```

binary: raw bytes. Literals 0x[ff d8], 0o[..], 0b[..]; spaces ok for
readability, incomplete bytes left-zero-padded. `into int` decodes.
```nu
# binary decodes to an integer
0x[ff] | into int # result: 255
```

nothing: the absence of a value; literal `null`. Printed bare it shows
nothing. `print` and `if`-without-else return null. NOT the same as a
MISSING cell (a red box in output); a missing cell errors on access unless
the cell-path member is marked optional with `?`.
```nu
# an optional cell-path member yields null instead of erroring
{a: 1}.b? # result: null
```

cell-path: a path into structured data, dot-separated ints (rows) and
strings (columns). A leading `$.` is required when binding to a variable.
`into cell-path` builds one programmatically from a list.
```nu
# a cell-path can be bound to a variable and reused with get
do {||
    let cp: cell-path = $.a.b
    {a: {b: 42}} | get $cp
} # result: 42

# cell-paths can be constructed at runtime
[a b 2] | into cell-path | describe # result: cell-path
```

glob: a filesystem pattern, distinct from the `glob` COMMAND. A command
that accepts a glob treats a bare/backtick word as a pattern but a quoted
string as a literal name. `**` recurses. `into glob` / `into string` cast.
Other signature-only shapes: path, directory (both expand ~ and .), error.
`any` matches every type; a variable typed `any` takes its value's real
type (describe never returns `any`). A mixed list is NOT `list<any>`: nu
derives a UNION of the element types, `list<oneof<...>>`.
```nu
# a mixed list describes as a union, not list<any>
[1 foo true] | describe # result: list<oneof<int, string, bool>>
```

## Variables & Mutability
`let` binds an IMMUTABLE variable (the default, idiomatic choice). `mut`
binds a mutable one. `const` binds a parse-time constant. Refer with
`$name`. `let $var = ...` is identical to `let var = ...`.

Immutable vars can be SHADOWED - redeclared, hiding the prior binding for
the rest of the scope. Reassigning with `=` errors unless the var is `mut`.
```nu
# an inner scope shadows without touching the outer binding
do {||
    let val = 42
    let inner = do { let val = 101; $val }
    [$val $inner]
} # result: [42 101]

def shadowing [] {
    let val = 42
    let val = $val + 1
    $val
} # result: 43
```

`mut` assignment operators: = += -= *= /= and ++= (append a value/list;
needs a list on one side). The arithmetic ops apply only where their
underlying operation is valid.
```nu
# mutate in place
do {||
    mut n = 42
    $n += 27
    $n
} # result: 69

def mutable_binding [] {
    mut n = 0
    $n += 1
    $n
} # result: 1
```

const is fully evaluated AT PARSE TIME, so it can feed parse-time keywords
(source, use, overlay use) that a `let` cannot (see Sharp Edges).

Closures and nested defs CAPTURE immutable variables and command params
only - never `mut` variables (compile error "Capture of mutable variable").
For accumulation prefer a filter (each/reduce) over a mut-var loop.
Variable NAMES cannot contain: . [ ( { + - * ^ / = ! < > & |
```nu
def cannot_capture_mut [] {
    [1 2 3] | reduce --fold 0 {|it, acc| $acc + $it }
} # result: 6
```

## Operators
Arithmetic: + - * / plus ** (pow), // (floor division), and mod. `/` on
ints yields a float when inexact. `//` floors toward negative infinity
and PAIRS with mod: a == (a // b) * b + (a mod b), so a positive divisor
gives a non-negative mod even for negative a.
```nu
# / is exact; // floors; ** raises
[(7 / 2), (7 // 2), ((-7) // 2), (2 ** 10)] # result: [3.5 3 -4 1024]

# mod pairs with floor division
(-7) mod 2 # result: 1
```

Bitwise OPERATORS on ints: bit-and, bit-or, bit-xor, bit-shl, bit-shr.
That is the whole infix set - bit-not and the rotates are NOT operators
(a bare `x bit-not` is a parse error); they ride the `bits` COMMAND
family (bits and/or/xor/not/shl/shr/rol/ror).
```nu
# bitwise and/or/xor
[(0b0110 bit-and 0b0011), (0b0110 bit-or 0b0011), (0b0110 bit-xor 0b0011)] # result: [2 7 5]

# shifts
[(1 bit-shl 4), (16 bit-shr 2)] # result: [16 4]

# rotate rides the command family, not an operator
1 | bits rol 1 # result: 2
```

Logical: and, or, not, xor. Keep boolean chains on ONE line - a
line-leading `and`/`or` parses as an implicit external (see Sharp Edges).
```nu
# the logical set
[(true and false), (true or false), (not true), (true xor false)] # result: [false true false true]
```

## Pipelines & IO
A pipeline is input | filter | output. Nu pipes structured VALUES, not just
text. Every command returns a value (possibly null); the last expression's
value is the pipeline result and is rendered if non-null.
```nu
def basic_pipeline [] { [1 2 3] | where {|x| $x > 1 } | math sum } # result: 5
```

`$in` is the pipeline input. RULES:
 1. In the FIRST position of a pipeline inside a closure/block, $in is the
    block input - and stays that value across the scope's first-position uses.
 2. Elsewhere in a pipeline, $in is the PREVIOUS expression's result.
 3. With no input, $in is null.
 4. After a `;`, $in cannot capture the previous STATEMENT (acts as no input).
```nu
# first position: $in is the block input
5 | do { $in + 1 } # result: 6

# later positions: $in is the prior stage
4 | $in * $in | $in / 2 # result: 8
```

`collect` forces a stream into a single value. `$in` on a stream currently
collects implicitly, but that is not guaranteed - use `collect` to be sure.
Prefer plain pipeline input over $in where possible (it avoids a
PipelineData->Value conversion and its memory/latency cost).

`;` separates statements; a newline is equivalent. Only the LAST value of
the whole expression is returned - earlier non-null values are discarded
(a common surprise: a stray expression mid-body is dropped). `print` forces
a value to display without it becoming the return value.
```nu
def semicolons [] { 40; 50; 60 } # result: 60
```

REDIRECTION (external commands). stdout is piped by default; stderr prints
to the terminal by default. File redirection (o/e/o+e; short for out/err/both):
  o> file    e> file    o+e> file       redirect a stream to a file
Pipe redirection: a plain `|` after an external pipes its stdout; `e>|`
pipes stderr, `o+e>|` pipes both - and these two work only on EXTERNALS.
`complete` runs an external to completion, capturing {stdout, stderr,
exit_code} as a record - the structured way to grab both streams.
`save <file>` is nu's `>`.
```nu
^cat missing.txt | complete # result: {stdout: "" stderr: "..." exit_code: 1}
```

External <-> Nu: data INTO an external is stringified to its stdin. Data
FROM an external arrives as bytes, auto-decoded to UTF-8 (falling back to
binary). Nu renders tables with borders before sending them to an
external - convert with `to text` first when piping to grep/etc. `decode <enc>` for
explicit decoding. `ignore` discards output (makes a command a statement).
```nu
ls | get name | to text | ^grep foo
0x[8a 4c] | decode shift-jis
```

## Closures & Blocks
A closure is an anonymous function: {|params| body}. Params optional; `{...}`
with no `|...|` is a closure only where a closure is expected (elsewhere
`{...}` is a record); `{||}` is the explicit empty closure. Invoke directly
with `do`. Closures CAPTURE immutable outer variables and can be stored,
listed, returned, and passed to commands. An EMPTY body does not return
null: a closure with no expressions passes its pipeline input through
unchanged ($in flows out), so `{||}` is the identity.
```nu
# invoke a closure with do
do {|a, b| $a + $b } 3 4 # result: 7

# an empty closure passes its input through
5 | do {||} # result: 5

# so under each, the empty closure is the identity map
1..3 | each {||} # result: [1 2 3]
```

Closure params can be typed. `$in` also carries pipeline input into most
closures - a filter like `each` sets $in to the current item.
```nu
# each sets $in to the current item
[1 2 3] | each { $in * 10 } # result: [10 20 30]

def typed_closure [] { do {|a: int, b: int| $a + $b } 34 8 } # result: 42
```

A BLOCK is the {...} used by keywords (if/for/while/loop/try). Unlike a
closure a block has no params, is not a value/type, and does not capture -
but CAN read and MUTATE outer `mut` variables and $env. That is why `mut`
accumulation must use a loop (block), not each (closure). A block opens a
new scope; if/else and try return their executed block's last value, loops
do not.
```nu
# a block can mutate an outer mut variable; a closure cannot
do {||
    mut x = 1
    if true { $x += 1000 }
    $x
} # result: 1001

def blocks [] {
    mut total = 0
    for x in [1 2 3] { $total += $x }
    $total
} # result: 6
```

## Control Flow
`if cond { } else { }` is an EXPRESSION (returns a value); without a
matching branch it returns null. `else` must ride the closing brace's line.
The code after `else` is an EXPRESSION, so `else 100` (bare) and chained
`else if` both work.
```nu
# if is an expression; else can be a bare expression
if false { 1 } else 100 # result: 100

def conditional [x] { if $x > 0 { 'positive' } else if $x == 0 { 'zero' } else { 'negative' } }
```

`match` compares a value to each pattern before `=>`. `_` is the catch-all;
`|` separates alternative patterns; patterns can DESTRUCTURE lists/records
and bind variables; `if <cond>` after a pattern is a guard. match is an
expression; the first matching arm wins.
```nu
# match with a catch-all
match 3 { 1 => "one", _ => "other" } # result: other

# alternative patterns with |
[red gray] | each {|c| match $c { red | green => "prim", _ => "?" } } # result: [prim "?"]

def pattern_match [rec] {
    match $rec {
        {name: 'bar', count: $n} if $n < 5 => ($n + 3)
        {name: _, count: $n} => ($n + 7)
        _ => 0
    }
}
```

for/while/loop are STATEMENTS: they return null and cannot sit mid-pipeline
("Statement used in pipeline"). Use them with `mut`/$env side effects; use
filters (each/where/reduce) when you want a returned value. break/continue
control loops. Unlike each, a loop does not fail the surrounding pipeline
when its body raises - handy for raising a clean error.
```nu
def loops [] {
    mut acc = []
    for x in [1 2 3] { $acc = ($acc | append ($x + 1)) }
    $acc
} # result: [2 3 4]
```

`return` exits a command/closure early with a value (rarely needed given
implicit return).
```nu
def early_return [x] { if $x > 0 { return 'pos' }; 'non-pos' }
```

## Working with Lists
A list is [a b c] - values separated by spaces, commas, and/or newlines.
0-indexed. A list equals one table column, so column filters work on it.
Access with `$list.<i>` or `get <i>` (get accepts a variable index).

Add/remove: insert <i> <v>, update <i> <v>, prepend, append, ++ (concat),
skip <n>, drop <n> (from end), first <n>, last <n>, take <n>.
```nu
# concatenate lists with ++
[1 2] ++ [3 4] # result: [1 2 3 4]

# first n slices from the front
[10 20 30 40] | first 2 # result: [10 20]

def mutating_lists [] { [1 2 3 4] | insert 2 10 } # result: [1 2 10 3 4]
```

Iterate/aggregate: each (map), where/filter (keep by predicate), reduce
(fold; --fold seeds the accumulator), any, all, enumerate (adds .index/.item).
```nu
# reduce folds a list to one value
[1 2 3 4] | reduce {|it, acc| $acc + $it } # result: 10

# enumerate pairs index with item
[a b] | enumerate # result: [{index: 0 item: a} {index: 1 item: b}]
```

Membership: `in`/`not-in` (element in list); `has`/`not-has` (list has
element). `flatten` lifts one level of nesting; `wrap` turns a list into a
single-column table. `is-empty`, `length`.
```nu
# has tests list membership
[1 2 3] has 2 # result: true

# flatten lifts one nesting level
[1 [2 3] [4 [5]]] | flatten # result: [1 2 3 4 [5]]

# wrap makes a one-column table
[1 2] | wrap n # result: [{n: 1} {n: 2}]

def membership [] { 'blue' in [red green blue] } # result: true
```

The spread operator `...` unpacks a list into a list literal (or a command's
rest args). It binds tightly: `...$x`, `...(expr)`, `...[a b]`, with NO space
before the operand, else `...` is a literal string.
```nu
# spread interleaves lists and values
[1 ...[2 3] 4] # result: [1 2 3 4]

def spreading [] { let x = [2 3]; [1 ...$x 4] } # result: [1 2 3 4]
```

Set ops treat lists as sets, returning UNIQUE elements: `union` (in
either), `intersect` (in both), `difference` (in input, not other).
`uniq` dedupes (--count adds a count column, --repeated keeps only dups,
--unique keeps once-only values, -i case-insensitive); `uniq-by <col>`
dedupes a table by column(s).
```nu
# union returns the unique elements of both
[1 2 3] | union [3 4] # result: [1 2 3 4]

# intersect keeps elements present in both
[1 2 3] | intersect [2 3 4] # result: [2 3]

# uniq --count tallies occurrences
[1 1 2] | uniq --count # result: [{value: 1 count: 2} {value: 2 count: 1}]

def set_ops [] { [1 2 3] | difference [2] } # result: [1 3]
```

`zip <other>` pairs two streams element-wise into a list of 2-lists.
`window <n>` slides an n-wide view (--stride steps by n, --remainder keeps
a short final window); `chunk-by {closure}` groups CONSECUTIVE items
sharing a key - unlike group-by, which pools ALL matching items regardless
of position.
```nu
# zip pairs two streams positionally
[1 2 3] | zip [a b c] # result: [[1 a] [2 b] [3 c]]

# window slides a fixed-width view
[1 2 3 4] | window 2 # result: [[1 2] [2 3] [3 4]]

# chunk-by groups runs of equal keys
[1 1 2 2 3] | chunk-by {|x| $x } # result: [[1 1] [2 2] [3]]
```

`generate {closure} <initial>` unfolds a stream (reduce's inverse): the
closure receives the state and returns a record with optional `out`
(emitted) and `next` (the next state); it stops when `next` is absent.
```nu
# generate unfolds until next is absent
generate {|x| if $x <= 3 { {out: $x, next: ($x + 1)} } } 1 # result: [1 2 3]
```

Predicate/positional slicing beyond first/last/skip/drop: `every <n>`
keeps every nth row (--skip inverts); `each while {c}` maps until the
closure returns null; `take while`/`take until`/`skip while`/`skip until`
slice by predicate; `drop nth <i>...` drops rows by index. `combinations
<k>`/`permutations` enumerate; `tee {c}`/`interleave` fork/merge streams.
```nu
# every keeps every nth row
[0 1 2 3 4 5] | every 2 # result: [0 2 4]

# each while stops at the first null
[1 2 3 -1 5] | each while {|x| if $x > 0 { $x * 10 } } # result: [10 20 30]

def slicing_more [] { [a b c d] | drop nth 1 } # result: [a c d]
```

## Working with Tables
A table is a list of records; any list command works on it. Two literal
forms: [[c1 c2]; [v1 v2] [v3 v4]] (header;rows) and [{c1: v1 c2: v2} ...].
Extracting one row (get 0) yields a RECORD.

select vs get: `select` returns a same-shaped SUBSET (table/list/record);
`get` returns the VALUE(S) at the cell-path (a list for multiple keys).
reject removes columns/rows; slice takes a row range; move --after/--before
reorders; rename renames columns.
```nu
# get returns values; multiple keys yield a list
{a: 1 b: 2 c: 3} | get b c # result: [2 3]

# select returns a same-shaped subset
{a: 1 b: 2 c: 3} | select b # result: {b: 2}
```

Combine: `append`/`++` stack rows (missing columns fill with the empty
cell); `merge` joins tables/records column-wise. insert adds a column,
update changes one, upsert does insert-or-update.
```nu
# merge joins records column-wise
{a: 1} | merge {b: 2} # result: {a: 1 b: 2}

# upsert inserts a missing key
{a: 1} | upsert b 2 # result: {a: 1 b: 2}
```

The `#` column is an internal 0-based display index, NOT a real column
(`get #`/`get index` fail). Creating a real `index` column replaces the
display. `enumerate | flatten` converts the internal `#` into a real,
movable `index` column (so sort-by carries it along). `table` RENDERS
structured data to a STRING (table -e expands, -i false hides the index,
-a N abbreviates); non-renderable inputs pass through unchanged.

`join <right> <left-on> [right-on]` is a SQL join (default --inner; also
--left/--right/--outer), with --prefix/--suffix to disambiguate right-side
columns. `merge` is a SHALLOW column merge; `merge deep` recurses into
nested records/tables (--strategy tunes how inner lists combine).
```nu
# join matches rows on a shared column
[[a b]; [1 2] [3 4]] | join [[a c]; [1 9]] a # result: [{a: 1 b: 2 c: 9}]

# merge deep recurses into nested records
{a: {b: 1}} | merge deep {a: {c: 2}} # result: {a: {b: 1 c: 2}}
```

`columns` lists a record/table's column names; `values` lists the values.
`update cells {closure}` applies a closure to EVERY cell (--columns limits
which). `rotate` turns a table 90 degrees so columns become rows (--ccw
reverses direction); `roll up/down/left/right` cyclically shifts rows/cols.
```nu
# columns lists the keys
{a: 1 b: 2} | columns # result: [a b]

# update cells transforms every cell
[[a b]; [1 2]] | update cells {|v| $v + 10 } # result: [{a: 11 b: 12}]

def cell_wide [] { [[a b]; [1 2]] | values } # result: [[1] [2]]
```

## Working with Strings
Six string forms:
  'single'        no escapes, no interpolation, cannot hold a single quote
  "double"        C-style escapes (\n \t \u{XXXX} \e ...), interpolation-ready
  r#'raw'#        no escapes; MAY hold single quotes; add #s to nest
  bareword        word chars only; in COMMAND position runs as an external
  `backtick`      bareword-with-spaces; also command/path in first position
  $"..." / $'...' interpolation (double form honors escapes)
```nu
# raw strings hold single quotes
r#'it's raw'# # result: it's raw
```

`char <name>` emits a special character: `char nl` (newline), `char tab`,
`char esep` (PATH separator), `char lparen`, etc. `char --unicode <hex>`
takes a codepoint; an ALL-DIGIT codepoint must be QUOTED ("41"), since a
bare 41 parses as an int and errors ("Can't convert to string").
```nu
# char --unicode maps a hex codepoint to its character
char --unicode "41" # result: A
```

Interpolation wraps expressions in (): `$"hi ($name)"`. Escape a literal
paren as \( in the double form. Interpolation is a common cast to string.
```nu
# interpolation evaluates the parenthesized expression
$'(2 + 2)' # result: "4"

# interpolate a variable
do {||
    let n = "Nu"
    $"hi ($n)"
} # result: hi Nu
```

Splitting: `split row <sep>` -> list; `split column <sep> [names...]` ->
table; `split chars` -> list of characters.
```nu
# split row makes a list
"a,b,c" | split row "," # result: [a b c]

# split column makes a table with named columns
"a|b|c" | split column "|" x y z # result: [{x: a y: b z: c}]
```

`str` subcommands: trim (-l/-r/-c char), contains (--ignore-case),
substring <range>, index-of, replace (FIRST match; -a/--all for every;
-r regex, -m multiline, -n stops $name capture-expansion in the
replacement), reverse, upcase/downcase, length, join; pad via `fill`.
```nu
# str replace changes only the first match by default
"foobar" | str replace "o" "0" # result: f0obar

# --all replaces every match
"foobar" | str replace --all "o" "0" # result: f00bar
```

`parse` extracts columns via a `{name}` template or --regex with named
groups. `from ssv|csv|tsv` and `detect columns` parse tabular text.
```nu
# parse pulls named fields out of a string
"Nu 0.80" | parse "{name} {ver}" # result: [{name: Nu ver: "0.80"}]

def parsing [] { 'all data is structured' | parse --regex '(?P<subj>\w+) is (?P<adj>\w+)' } # result: [{subj: data adj: structured}]
```

Comparison operators on strings: == != < <= > >= (case-sensitive), =~/!~
(regex match, rust regex crate; prefix with (?i) for case-insensitive),
starts-with/ends-with. `ansi <name>` colors text; close with `ansi reset`.
```nu
# regex match operator
"APL" =~ '^\w{0,3}$' # result: true

def comparisons [] { 'JavaScript' starts-with 'Java' } # result: true
```

Case conversions each re-split words on spaces and case boundaries first:
`str snake-case`, `camel-case`, `kebab-case`, `pascal-case`,
`screaming-snake-case`, `title-case`, `capitalize` (first letter only),
plus `upcase`/`downcase`.
```nu
# snake-case joins words with underscores
"hello world" | str snake-case # result: hello_world

# camel-case lowercases only the first word
"hello world" | str camel-case # result: helloWorld
```

`str expand` does brace expansion; `str distance` is Levenshtein edit
distance; `str stats` reports lines/words/chars/bytes/graphemes counts.
`detect type` infers and casts a string to its apparent Nu type.
```nu
# str expand enumerates brace combinations
"A{1,2}B" | str expand # result: [A1B A2B]

# str distance is Levenshtein
"kitten" | str distance "sitting" # result: 3

# detect type infers a string's type
"-5" | detect type | describe # result: int

def string_analysis [] { "one two three" | str stats | get words } # result: 3
```

## Records & Cell-Paths
A record is {key: value ...}, pairs separated by commas/spaces/newlines.
Keys are strings, UNIQUE (last wins), and keep insertion order. A record is
one table row, so row commands (update, insert, merge...) work on it. A
COMPOUND value (with operators) must be PARENTHESIZED: {k: (1 + 1)}.
```nu
# compound record values need parentheses
{k: (1 + 1)} # result: {k: 2}
```

Cell-path navigation: record uses string keys, list uses int indices, table
combines them (`$data.temps.2.1`). For a single table cell the row/column
order can swap, but nested indices are order-significant. Keys with spaces
or numeric-looking keys must be quoted: `$rec."key x"`, `$rec."1"`.

Missing/optional data: append `?` to a cell-path member so a missing
row/column yields null instead of erroring. `default <val> <col>` fills
missing/null cells; `compact` drops nulls from a list; `reject` removes.
```nu
# optional operator suppresses the missing-cell error
{a: 1}.b? # result: null

# default fills missing/null cells
[[a]; [1] [null]] | default 0 a | get a # result: [1 0]

def missing_data [] { [1 null 2 null 3] | compact } # result: [1 2 3]
```

Iterate a record with `items {|key, value| ...}`, or `transpose <k> <v>` to
a table then use table filters. (each/where iterate lists/tables, not records.)
```nu
# items iterates key/value pairs
{a: 1 b: 2} | items {|k, v| $"($k)=($v)" } # result: ["a=1" "b=2"]

# transpose turns a record into a two-column table
{a: 1 b: 2} | transpose key val # result: [{key: a val: 1} {key: b val: 2}]
```

Extend a record via `merge {..}` or the spread `{...$rec, extra: 1}`.
`metadata <value>` reads pipeline metadata (notably `.span`, the source
location) - the basis of custom error labels.

## Modules & Overlays
A module groups defs/aliases/consts/externs/env/submodules. Sourced from a
`<name>.nu` FILE (filename = module name) or a `<dir>/mod.nu` DIRECTORY
(dir = module name). `use <path> [members]` imports; the import pattern
controls scope: `use std/log` -> `log` + subcommands; `use std/formats *`
-> the definitions directly; `use std/math PI` -> just PI. Relative paths
also search $NU_LIB_DIRS (const), then $env.NU_LIB_DIRS (deprecated).

Only `export`ed items are visible to importers; unmarked defs are private.
Export keywords: export def / export alias / export const / export extern /
export module (declares a submodule) / export use (re-exports another
module's definitions into THIS module) / export-env (env setup). A def
cannot share its module's name - name it `main` and it imports AS the
module name. `main` imports only via `use <mod>`, `use <mod> *`, or
`use <mod> main`. Importing a whole module PARSES it entirely; the slash
form `use std/help` beats the space form `use std help` (which first loads
the parent). A module's constants are reached via a record named for the
module: `use std/math` then `$math.PI`.

`hide <name>` removes a command/alias for the rest of the scope (accepts
import patterns; hiding a module hides its prefixed exports). `hide-env`
removes an env var (scoped).
```nu
use std/assert; hide assert
```

OVERLAYS are named, swappable layers of definitions built on modules. The
default overlay is `zero`. `overlay use <mod>` activates one (runs its
export-env); `overlay list` shows the stack; `overlay hide` removes the last
(or a named one). New defs record into the LAST active overlay and are
remembered even after hide - re-activating restores them (context swapping):
  overlay use --prefix m   keep exports behind the module name
  overlay use m as n       rename the overlay
  overlay new scratch      empty overlay to record into
  overlay hide --keep-custom / --keep-env [VARS]   keep additions on hide
Overlays stack; the last active wins on collisions; re-`overlay use` tops one.
```nu
overlay use spam; overlay hide spam
```

## Custom Commands & Attributes
`def name [params] { body }` defines a first-class command. IMPLICIT RETURN:
the last expression's value is returned - no return/echo needed. A trailing
STATEMENT (for/while) makes it return null; end with `ignore` to return
nothing on purpose. Names may contain most chars and spaces; NOT pure
numbers, filesizes, or leading -/{/(. The name must be known at parse time
(not a variable). A def name with a space is a SUBCOMMAND (def "str mine").
```nu
# implicit return: the last expression is the result
do {||
    def add-one [x] { $x + 1 }
    add-one 41
} # result: 42
```

Parameters: positional (required by default; `name?` optional -> null when
absent; `name = default` gives a default and makes it optional). Types
annotate with `name: type` (untyped = any), checked at PARSE time. Optional
params must follow required ones.
```nu
# typed parameter with a default
do {||
    def greet [name: string = "Nu"] { $"Hello, ($name)!" }
    greet
} # result: Hello, Nu!
```

Flags: `--flag` (value flag; `--flag (-x)` adds a short form; absent = null).
A flag with NO type is a SWITCH (true present, false absent); annotating a
flag `bool` is disallowed. Set a switch explicitly with `--flag=true` (a
SPACE would pass a positional). Dashes map to underscores in the variable
(`--all-caps` -> `$all_caps`).
```nu
# a switch flag is boolean by presence
do {||
    def shout [s: string, --caps] { if $caps { $s | str upcase } else { $s } }
    shout hi --caps
} # result: HI
```

Rest params `...names: type` collect extra positionals into a list. Spread a
list into them with `...$list`. `def --wrapped` collects unknown flags/args
too, for wrapping an external command.
```nu
# rest params collect variadic args
do {||
    def sum-all [...ns: int] { $ns | math sum }
    sum-all 1 2 3 4
} # result: 10
```

Input/output signature: `def name [params]: IN -> OUT { }` narrows pipeline
types (parse-checked). Multiple pairs go in [ ... ]. `nothing -> ...` means
no pipeline input; `... -> nothing` means returns null. Comments just above
`def` become the help description; a single-line comment after a param
documents it (needs a space before the `#`). `def --env` lets the command's
$env and `cd` changes PERSIST in the caller.
```nu
def io_signature []: nothing -> string { "typed pipeline" } # result: typed pipeline
```

ATTRIBUTES enrich a def, written `@name ...` immediately above it. They must
be CONTIGUOUS with the def - ONLY attribute lines may sit between the first
attribute and the def. A blank line OR a comment between them breaks the
association: a parse error, AttributeRequiresDefinition ("Attributes must be
followed by a definition"). So keep each @example flush against its def.
  @example "desc" { code } --result <val>   help example (and doctest)
  @deprecated "msg" / @category "label" / @search-terms "a" "b"
The @example/@category/etc. builtins live under `attr`; a custom attribute
is a command named `attr <x>` (see std/testing's `attr test`).
```nu
@deprecated "kept only as an illustration"
@category "demo"
def attributed [] { "see the attribute lines above" }
```

SCRIPTS: `nu file.nu` runs a file in a fresh instance; `source file.nu` runs
it in the current one. Definitions are hoisted (they run first, so their
order in the file is free). A special `main` command receives the script's
CLI args (`nu file.nu <args>`) and runs after other top-level code; unlike a
module, a script's `main` need NOT be exported. `def "main sub"` makes
subcommands, but an (even empty) `def main []` must exist to expose them.
Untyped script args parse by apparent type; annotate to force one. Shebang:
`#!/usr/bin/env nu` (use `-S nu --stdin` to receive piped stdin as `$in`).
```nu
def main [x: int] { $x + 10 }
```

EXTERN / export extern gives an EXTERNAL command a typed signature - enabling
parse-time type checking, completions, and syntax highlighting for it. Same
shape as `def` (flags, positionals, `?` optional, `...rest: path`,
`@completer`). Limits: it cannot force flag/positional ordering, cannot
require `=`-style flags, single-dash long flags are unrepresentable, and a
`^`-run external is not matched to its extern.
```nu
export extern ssh [dest: string, -p: int]
```

## Error Handling
`error make { msg, label?: {text, span}, help? }` raises an error, halting
the current expression. The span (from `(metadata $arg).span`) underlines
the offending argument in the caller's source. `error make -u` omits it.
```nu
def custom_error [x] {
    let span = (metadata $x).span
    error make {msg: "this is fishy", label: {text: "here", span: $span}}
}
```

`try { } catch { }` catches errors (builtin and external) and resumes. The
try block returns null on error, else its value. `catch` receives the error
as a RECORD (fields msg, debug, raw, rendered, details) - NOT an `error`
type value. `try` with no catch just swallows the error.
```nu
# catch supplies a fallback value
try { 1 / 0 } catch { "caught" } # result: caught

# the caught error is a record, not an error value
try { error make {msg: "x"} } catch {|e| $e.msg } # result: x

def try_catch [] { try { ^nonexistent } catch { "handled" } } # result: handled
```

`$env.LAST_EXIT_CODE` holds the last external's exit code (POSIX `$?`); in a
catch block `$e.exit_code` gives an external's code. The `error` TYPE exists
but can never be assigned (any error terminates the binding), so it is
useless as an annotation. In a pipeline, `error make` inside an `each`
closure yields BOTH a pipeline-eval error and the custom error; a `for` loop
raises only the custom error - loops are the cleaner place to raise.

## Environment
`$env` is a special mutable record of environment variables (any type/value).
Set with `$env.VAR = val`; read with `$env.VAR`. `$env` is CASE-INSENSITIVE
on every OS ($env.PATH == $env.Path); force sensitivity with
`$env | get --sensitive`. Access a maybe-unset var with `$env.VAR?` (null,
not error), or test `"VAR" in $env`.

Set many at once with `load-env {A: 1 B: 2}`. Set one temporarily for a
block with `with-env {FOO: BAR} { ... }`, or the shorthand `FOO=BAR cmd`.
Remove a scoped var with `hide-env VAR`.
```nu
def loading [] { with-env {FOO: BAR} { $env.FOO } } # result: BAR
```

SCOPING: env changes live only in the current block/scope and are lost on
exit - `cd` (which sets $env.PWD) follows the same rule. A command keeps its
env changes in the caller ONLY if defined `def --env`. This scoping is what
makes `par-each` over subdirectories safe.
```nu
def scoping [] {
    $env.FOO = "BAR"
    do { $env.FOO = "BAZ" }
    $env.FOO
} # result: BAR
```

$env.PATH is auto-converted from the inherited string to a `list` at startup
and back to a string per external. `$env.ENV_CONVERSIONS` holds
{VAR: {from_string, to_string}} closures controlling such conversions
(applied AFTER env.nu/config.nu load). `$env.config` is the main config
record (documented via `config nu --doc`).

## Shell Configuration
Startup files load in order from $nu.default-config-dir: env.nu, config.nu,
$nu.vendor-autoload-dirs/*.nu, $nu.user-autoload-dirs/*.nu, then login.nu
(login shells only). `config nu` / `config env` open them in the editor
($env.config.buffer_editor or $env.EDITOR); `config nu --doc` documents every
setting; `--default` shows the internal defaults. Startup flags: `-n` skips
user config, `--no-std-lib` skips std, `-l`/`--login`, `-c`/`--commands`,
`-I`/`--include-path`, `--config`/`--env-config`, `--plugins '[...]'`.

`$env.config` is the settings record - populated by Nushell, NOT inherited
from the parent nor exported to children. Set INDIVIDUAL keys (assigning the
whole record resets everything else): e.g. `$env.config.show_banner = false`,
`$env.config.edit_mode = 'vi'`, `$env.config.buffer_editor = "hx"`,
`$env.config.table.mode = "rounded"`, `$env.config.recursion_limit = 50`,
`$env.config.plugin_gc.default.stop_after = 10sec`. Parse-time search paths
are the CONSTANTS `const NU_LIB_DIRS = [...]` (modules/scripts) and
`const NU_PLUGIN_DIRS = [...]`, settable and usable inside config.nu.

PROMPT is set via env vars, each a string, a no-arg closure, or null (revert
to default): $env.PROMPT_COMMAND, PROMPT_COMMAND_RIGHT, PROMPT_INDICATOR,
PROMPT_INDICATOR_VI_NORMAL / VI_INSERT, PROMPT_MULTILINE_INDICATOR. The
TRANSIENT_PROMPT_* variants replace the prompt AFTER a line runs (for clean
copy/paste).

HOOKS (interactive/REPL only) run code at events, configured under
$env.config.hooks: pre_prompt, pre_execution, env_change (keyed by var name;
each closure receives |before, after|), display_output (renders results),
command_not_found. A hook is a block (env changes persist, like def --env),
a record {condition, code}, or a string (evaluated like REPL input, so it can
define commands/aliases). `commandline` reads the pending line in pre_execution.
```nu
$env.config.hooks.env_change.PWD = [{|before, after| $after }]
```

COMPLETIONS: a completer is a command returning the suggestions; attach it to
an argument's type with `type@completer` (or `type@"quoted name"`). Return a
list, or a record {options: {sort, case_sensitive, completion_algorithm:
prefix|substring|fuzzy, match_description}, completions}; items may be
{value, description, style}. Return null to fall back to file completion, []
to suppress. A completer with `[context: string, position?: int]` sees the
line so far. `@` binds at PARSE time (re-import to refresh). `extern`s take
completers too. An EXTERNAL completer (carapace, fish, ...) goes in
`$env.config.completions.external = {enable, max_results, completer: {|spans| }}`.
```nu
def animals [] { [cat dog] }; def pet [a: string@animals] { $a }
```

LINE EDITOR (Reedline): a commandline spans lines when a bracket/paren is
open, after a trailing `|`, or via Alt/Shift+Enter; Ctrl+O edits it in
$env.EDITOR. `$env.config.edit_mode` is 'emacs' (default) or 'vi'; bindings
live in `$env.config.keybindings` as records {name, modifier, keycode, mode,
event}. History recall: `!!`, `!$`, `!<n>`, Ctrl+R search.

## Jobs & Processes
Background jobs are THREADS, not processes (experimental). `job spawn
{closure}` -> an integer id running in the background. `job list`,
`job kill <id>`, `job unfreeze [id]` (resume a Ctrl-Z frozen external).
`job send <id>` / `job recv` pass data between jobs; the main thread is id 0.
All jobs terminate when the shell exits (no `disown`).
```nu
let id = job spawn { sleep 10sec }
```

`timeit { ... }` measures a closure's wall time (a duration); std `bench`
gives statistical timing; `sleep <duration>` pauses.

External commands: prefix any string/variable with `^` to run it as external
(`^ls` runs /bin/ls, bypassing nu's `ls`); `run-external` does the same with
more control; `complete` runs one capturing stdout/stderr/exit_code. `par-each`
runs a closure over pipeline items in PARALLEL (threads) - the parallel twin
of `each`; results may return out of order, so sort afterward if order
matters. Immutability is what makes it safe.
```nu
def parallel [] { [1 2 3 4] | par-each {|x| $x * $x } | sort } # result: [1 4 9 16]
```

## Command Families
Nu's builtins cluster into families sharing a prefix; they transform
structured VALUES, not text:
  into <type>   cast (into int/float/string/bool/datetime/duration/filesize/
                binary/record/cell-path/glob/...). Quote all-digit args.
  str <sub>     string ops (help str).
  path <sub>    parse/join/expand/basename/dirname/type/exists (some const).
  bytes / bits  binary manipulation; encode / decode for text encodings.
  from <fmt> / to <fmt>   parse/serialize csv, json, yaml, toml, nuon, xml,
                ini, ... `open` auto-picks a `from` by extension.
  math <sub>    sum/avg/min/max/stddev/median/mode/round/abs/... over data.
  split <sub>   row/column/chars/list.  url / query db "SQL" (opened sqlite).
  http <verb>   native, structured http (attaches response metadata).
  date now / format date "%..." / into datetime.  seq / seq char / seq date.
  scope / help / describe / inspect / metadata / ast / view   introspection.
```nu
# into casts between types (quote all-digit args)
"123" | into int # result: 123

# math aggregates a collection
[1 2 3 4] | math sum # result: 10

help commands | where category == filters
```

`open <file>` loads and PARSES by extension into structured data (csv, eml,
ics, ini, json, nuon, ods, ssv, toml, tsv, url, vcf, xlsx, xml, yaml, and
sqlite by content regardless of extension); a custom `from ext` extends it.
`open --raw` gives bytes/text. `save <file>` serializes by extension (nu's
`>`). `lines` splits raw text into a list of lines.
```nu
open data.json | get version
```

NUON (Nushell Object Notation): valid nu describing data, a superset of JSON
(comments allowed, commas optional). `to nuon` / `from nuon`. Cannot
serialize closures/blocks. Records display vertically; `to nuon` is one line.
```nu
def nuon [] { {name: "Sam" rank: 10} | to nuon } # result: {name: Sam, rank: 10}
```

`hash md5` / `hash sha256` digest a string or binary to a hex string.
`random` generates values: uuid, chars, int/float [range], bool, binary
<len>, pass (secure password). `into value` collapses a plugin/custom
value (a polars frame, a stored handle) back into base Nu data.
```nu
# hash sha256 is deterministic hex
"abc" | hash sha256 # result: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
```

URL family: `url parse` -> a record (scheme/host/port/path/query/...);
`url join` rebuilds one; `url encode`/`decode` percent-code a string;
`url build-query` makes a query string from a record (space -> +);
`url split-query` parses one back into a key/value table.
```nu
# url build-query percent-encodes a record
{a: 1 b: "x y"} | url build-query # result: a=1&b=x+y

def url_family [] { "https://ex.com/p?q=1" | url parse | get host } # result: ex.com
```

SQLite: `open db.sqlite` reads a file DB (as a lazy structure); `query db
"SQL"` runs SQL over it; `schema` shows its tables; `into sqlite <file>`
writes a table out. `stor` is a session in-memory DB (stor create / insert
/ open / reset). SQLite is detected by content, so extension is irrelevant.
```nu
open my.db | query db 'select * from t'
```

Format family (structured -> string): `format pattern "{col}"` fills a
template per row; `format date`/`format duration`/`format filesize`/
`format number`/`format bits` render one typed value. Introspection:
`nu-check` validates source, `view source <cmd>` prints a def's body,
`view ir`/`ast`/`explain`/`debug profile` expose the compiled/parsed form,
`describe --detailed` returns a structural record instead of a type string.
```nu
# format pattern fills a template per row
[[name]; [bob]] | format pattern "hi {name}" # result: ["hi bob"]
```

Reshaping data: `group-by <col>` -> a record of tables keyed by the value
(`group-by <type> <col>`, e.g. `group-by date d`, buckets by that type);
`transpose [names]` swaps rows/columns (record<->table); `transpose
--header-row` uses row 0 as headers, and with a two-column table
`--as-record` yields one record; `rename <new...>` renames columns;
`headers` promotes the first row to column names; `chunks <n>` splits into
n-sized subtables; `slice <range>` takes a row range; `drop column [n]`
drops trailing columns; `into <type> <col>` casts one column in place.
```nu
# chunks splits into fixed-size groups
[1 2 3 4 5] | chunks 2 # result: [[1 2] [3 4] [5]]

# transpose --header-row --as-record rebuilds a record
[[k v]; [a 1] [b 2]] | transpose --header-row --as-record # result: {a: 1 b: 2}

# rename renames columns positionally
[[a b]; [1 2]] | rename x y # result: [{x: 1 y: 2}]

# headers promotes the first row to column names
[[column0 column1]; [name age] [bob 3]] | headers # result: [{name: bob age: 3}]

# into <type> casts a single named column
[[n]; ["3"] ["4"]] | into int n | get n # result: [3 4]

# slice takes a row range
[a b c d] | slice 1..2 # result: [b c]

def reshaping [] { [{t: a} {t: a} {t: b}] | group-by t } # result: {a: [{t: a} {t: a}] b: [{t: b}]}
```

`http get URL` / `http post URL BODY` and friends are native and structured:
  -r/--raw text not parsed; --headers [k v]; -u/-p auth; -t timeout;
  --content-type "multipart/form-data" (fields may be `open -r file` /
  `into binary`); --full for {body, headers, status}; --allow-errors;
  --unix-socket. Response metadata: `... | metadata | get http_response.status`,
  or `metadata access {|m| ...}` to read status/headers WHILE streaming the body.
```nu
http get https://example.com
```

Filesystem commands are BUILT-IN and cross-platform (ls, cd, mkdir, mv, cp,
rm, open, save, glob, watch, du, start). Notably: `mkdir` is `-p` by default
(creates parents, no error if present; there is NO -p flag); `cd`/bare-path
navigate, and `..`/`...`/`....` go up 1/2/3 levels; `rm -t` trashes, `rm -r`
recurses; `start <file>` opens with the OS default app; `watch <dir> --glob
<pat> {|| ... }` re-runs on change. Glob patterns: `*` one level, `**`
recurses, `?` one char, `[...]` a class; quote to pass a literal (backtick
does NOT escape a glob); build one with `into glob` or spread `...(glob $p)`.
```nu
ls **/*.nu | where size > 10kb
```

PLUGINS extend the binary. Install the executable, register it once with
`plugin add <nu_plugin_file>` (by FILENAME), then `plugin use <name>` (by
name, no nu_plugin_ prefix) to load it now; registered plugins auto-load at
startup. `plugin list` / `plugin stop <name>` / `plugin rm <name>`.
`plugin use` is a PARSER keyword (cannot add+use in one script - use
`nu --plugins '[...]'`). Core plugins shipped as separate executables
alongside official releases (register before use): polars (DataFrames),
formats (from/to eml/ics/ini/plist/vcf), gstat (git status), query
(query json/web/xml/webpage-info - note `query db` is BUILT-IN), inc.
List loaded plugins with `plugin list`, or their commands with
`scope commands | where type == "plugin"`.
```nu
plugin add nu_plugin_polars; plugin use polars
```

## Std Library
The standard library ships built-in, loaded but NOT auto-imported. Import
per-submodule with the SLASH form for speed:
  use std/assert   assert + subcommands (assert equal/less/greater/error/
                   length/"str contains"; `assert not`).
  use std/iter     iter find / find-index / intersperse / scan / filter-map /
                   flat-map / zip-with / zip-into-record (filter helpers).
  use std/dt *     datetime-diff, pretty-print-duration.
  use std/formats *   from/to ndjson, jsonl, ndnuon (streaming JSON lines).
  use std/math     $math.PI/E/TAU/PHI/GAMMA (or `use std/math *` for bare $PI).
  use std/log      log debug/info/warning/error/critical (stderr; level via
                   $env.NU_LOG_LEVEL; format via $env.NU_LOG_FORMAT).
  use std/dirs     directory stack: dirs add/next/prev/drop/goto.
  use std/bench    statistical benchmarking of closures.
  use std/input    input display / input listen (UI events).
  use std/help     alternative help with completions.
`use std *` loads everything (slow - avoid in scripts). Top-level std adds
`path add`, `repeat`, `null-device`, `ellie`, `banner`. `nu --no-std-lib`
disables it entirely; std-rfc is the separate CANDIDATE staging module.
```nu
# assert equal passes silently on equal values
do {||
    use std/assert
    assert equal (1 + 1) 2
    "ok"
} # result: ok
```

`use std/math` gives $math.PI etc. as exact f64 literal constants.
```nu
# std math exposes constants
do {||
    use std/math
    ($math.PI > 3.14 and $math.PI < 3.15)
} # result: true

def math_constants [] { use std/math *; $PI } # result: 3.141592653589793
```

std/iter adds functional helpers builtins lack: `iter find` returns the
first match or null; `iter scan` returns reduce's intermediate steps.
```nu
# iter find returns the first predicate match
do {||
    use std/iter
    [shell abc around] | iter find {|e| $e starts-with "a" }
} # result: abc

def iter_helpers [] { use std/iter; [1 2 3] | iter intersperse 0 } # result: [1 0 2 0 3]
```

## Coming from Other Shells
Nushell is NOT Bash/POSIX/PowerShell. Redirection differs: `>` is the
greater-than OPERATOR, not a redirect. Save output with `| save file` (or
`o> file` for an external's stdout); append with `| save --append` (or
`o>> file`); discard with `| ignore` (Bash `>/dev/null`); combine streams
with `o+e>| ...` (Bash `2>&1 |`). There is no `eval`; filesystem commands
(ls/cd/cp/mv/rm/mkdir) are built-in and cross-platform, not separate binaries.
```nu
"hi" | save out.txt   # bash: echo hi > out.txt
```

Command map (Bash / PowerShell -> Nu):
  find . -name X / Get-ChildItem -Recurse   -> ls **/X
  ls -d */                                   -> ls | where type == dir
  cat f      / Get-Content                   -> open --raw f   (open f parses it)
  sed        / -replace                      -> str replace
  grep       / Select-String                 -> where ... =~ ... / find
  man X      / Get-Help                      -> help X   (help commands; help --find)
  cmd1 && cmd2                               -> cmd1; cmd2
  echo $?    / $LASTEXITCODE                 -> $env.LAST_EXIT_CODE
  export X=Y / $env:X=Y                      -> $env.X = Y
  unset X    / Remove-Item Env:X             -> hide-env X
  ${X:-d}                                    -> $env.X? | default "d"
  read v     / Read-Host                     -> let v = input   (input -s for secret)
  type X     / Get-Command X                 -> which X
  xargs cmd  / $_.Prop                       -> cmd ...$in / get col (no .Prop access)

PowerShell passes .NET objects with `.Property` access; Nushell passes
structured DATA with none - use `get <col>`, `select`, or `$it.col`. A
foreign shell's env cannot be sourced; capture it by running that shell and
parsing its `env`/output, e.g.:
```nu
^ssh-agent -c | lines | parse "setenv {name} {value};"
| transpose --header-row | into record | load-env
```

## Sharp Edges
PARSE then EVAL, as two whole-program stages - Nushell has NO `eval`. All
code to run must be known at parse time. Parse-time keywords (source, use,
overlay use, hide, source-env) need path/args KNOWN at parse - a runtime
`let` path fails ("not_a_constant"); a `const` works because consts evaluate
during parsing. Generating a file then sourcing it in one expression cannot
work. In the REPL each entry is its own parse+eval, so two SEPARATE lines
can do what one expression cannot.

A record value that is a compound expression must be PARENTHESIZED, else the
parser mis-reads it: `{k: (1 + 1)}` works, `{k: 1 + 1}` is a parse error.

`%name` forces the INTERNAL/builtin command even when a def/alias shadows it;
`^name` forces the EXTERNAL. Aliases are POSITION-DEPENDENT: to replace a
builtin, back it up with an alias FIRST, then shadow with `def` (a
self-referential `def ls [] { ls }` hits the recursion limit).
```nu
# %name calls the builtin past a shadowing def
do {||
    def length [] { -1 }
    [1 2 3] | %length
} # result: 3
```

`sort` NEVER errors on mixed types (numbers first, then strings, ...) and is
STABLE. `sort -i` case-insensitive, `sort -n` natural (numeric strings).
`sort-by <cell-path>` for tables; `sort-by {closure}` for a key; `sort-by -c
{|a, b| ...}` for a custom order. Records sort by key (`-v` by value).
```nu
# sort orders numbers before strings without erroring
[b 2 a 1] | sort # result: [1 2 a b]
```

More edges (no runnable result, but load-bearing):
 - `else` must sit on the closing brace's line; a line-leading `else` parses
   as an external command and fails at eval with no clear diagnostic.
 - Reserved keywords (match if else let mut const def for while loop break
   continue return try catch do where use source module export ...) cannot be
   BARE record keys - quote them: {"match": 1}.
 - `where` on a list wants an explicit closure: `where {|x| $x > 1}`. A bare
   `$it` expression on a list of strings can fail "Input type not supported".
 - `open file.json/.toml/.md` AUTO-PARSES by extension; for raw text use
   `open --raw | decode`, else `lines`/`str ...` fail "Input type not supported".
 - A BAREWORD in command position runs as an EXTERNAL, not a string; `true`/
   `false` are keywords there. Quote strings used as values.
 - `path exists` follows symlinks (a dangling link reads absent); test a link
   itself with `path type == "symlink"`.
 - Immutable is the default; `mut` cannot be captured by closures. Loops
   return null and cannot sit mid-pipeline. A `;` resets $in to null.
