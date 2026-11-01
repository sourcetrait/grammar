#!/usr/bin/env -S nu --no-config-file --no-std-lib --stdin
# Claude Code's statusline from its session JSON on stdin, each segment left
# out when its field is: `<project>: <model> (<effort>) <tokens> [<5h%>
# <HHMM>] {<7d%>}`, the reset clock in UTC.

def main [] {
    let s = $in | collect | from json
    let model = $s.model?.display_name?
    let effort = $s.effort?.level?
    let n = $s.context_window?.total_input_tokens?
    let five = $s.rate_limits?.five_hour?
    let seven = $s.rate_limits?.seven_day?.used_percentage?
    let body = [
        (if $model != null { $model | str replace 'M context' 'M' })
        (if $effort != null { $"\(($effort))" })
        (if $n == null { null } else if $n >= 1_000_000 { $"(($n + 500_000) // 1_000_000)M" } else if $n > 999 { $"($n // 1000)k" } else { $"($n)" })
        (if $five.used_percentage? != null {
            let at = $five.resets_at?
            let clock = if $at == null { '' } else { $" ($at * 1_000_000_000 | into datetime | format date '%H%M')" }
            $"[($five.used_percentage | math floor)%($clock)]"
        })
        (if $seven != null { $"{($seven | math floor)%}" })
    ] | compact | str join ' '
    let dir = $s.workspace?.project_dir?
    print (if $dir == null { $body } else { $"($dir | path basename): ($body)" })
}
