//! A known_hosts file read for one host's keys, as OpenSSH's client matches them.
use crate::*;

/// `host`'s keys in a known_hosts file's text, as OpenSSH lines, each once.
pub fn known_host_keys(
    text: &str,
    host: &str,
    port: u16,
) -> Result<Vec<String>, String> {
    let name = known_hosts_name(&host.to_ascii_lowercase(), port);
    let mut found: Vec<(String, kh::KeyData)> = Vec::new();
    let mut revoked: Vec<kh::KeyData> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.first().is_none_or(|first| first.starts_with('#')) {
            continue;
        }
        let entry: kh::Entry = fields
            .join(" ")
            .parse()
            .map_err(|e| format!("line {} isn't a known_hosts line: {e}", index + 1))?;
        if !host_matches(entry.host_patterns(), &name) {
            continue;
        }
        let data = entry.public_key().key_data().clone();
        let at = if entry.marker().is_some() { 2 } else { 1 };
        let key = fields.get(at..at + 2).map(|pair| pair.join(" ")).unwrap_or_default();
        match entry.marker() {
            Some(kh::Marker::CertAuthority) => {}
            Some(kh::Marker::Revoked) => revoked.push(data),
            None if pinned_key(&key).is_ok() && !found.iter().any(|(_, seen)| *seen == data) => {
                found.push((key, data));
            }
            None => {}
        }
    }
    Ok(found
        .into_iter()
        .filter(|(_, data)| !revoked.contains(data))
        .map(|(key, _)| key)
        .collect())
}

/// Whether a line's host field takes `name`, as OpenSSH's `match_hostname` does.
fn host_matches(
    patterns: &kh::HostPatterns,
    name: &str,
) -> bool {
    match patterns {
        kh::HostPatterns::HashedName { salt, hash } => {
            let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, salt);
            ring::hmac::verify(&key, name.as_bytes(), hash).is_ok()
        }
        kh::HostPatterns::Patterns(patterns) => {
            let mut matched = false;
            for pattern in patterns {
                let (negated, pattern) = match pattern.strip_prefix('!') {
                    Some(pattern) => (true, pattern),
                    None => (false, pattern.as_str()),
                };
                if glob(&pattern.to_ascii_lowercase(), name) {
                    if negated {
                        return false;
                    }
                    matched = true;
                }
            }
            matched
        }
    }
}

/// OpenSSH's `match_pattern`: `*` takes any run of characters, `?` any one.
fn glob(
    pattern: &str,
    text: &str,
) -> bool {
    let (pattern, text) = (pattern.as_bytes(), text.as_bytes());
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        match pattern.get(p) {
            Some(b'*') => {
                star = Some((p, t));
                p += 1;
            }
            Some(&c) if c == b'?' || c == text[t] => {
                p += 1;
                t += 1;
            }
            _ => match star {
                Some((at, from)) => {
                    p = at + 1;
                    t = from + 1;
                    star = Some((at, from + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|c| *c == b'*')
}
