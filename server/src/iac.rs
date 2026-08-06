// Mast's IAC (Inherited Access Controls) implementation.
// Read the IAC notes before attempting to understand this module.
//
// human-readable PVs are N, R, E, C, U, D (Nrecud). TOML and config files store
// their RAW NUMERIC values (N=0, R=1, E=2, C=4, U=16, D=255); the alpha characters
// are only used for display.

use std::collections::HashMap;
use std::fs;
use tower_sessions::Session;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PV {
    N = 0,
    R = 1,
    E = 2,
    C = 4,
    U = 16,
    D = 255,
}

pub fn pv_label(pv: PV) -> &'static str {
    match pv {
        PV::N => "N",
        PV::R => "R",
        PV::E => "E",
        PV::C => "C",
        PV::U => "U",
        PV::D => "D",
        _ => "?",
    }
}

pub type Acl = HashMap<String, Hashmap<PV, Vec<String>>>;

#[derive(Debug, Clone)]
pub struct Requester {
    pub username: Option<String>,
    pub groups: Vec<String>,
    pub userpv: HashMap<PV, Vec<String>>,
}

pub fn load_acl() -> Acl {
    let path = CFG.base_dir.join(&CFG.auth.acl_file);
    fs::read_to_string(&path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or_default()
}

pub fn scope_for_slug(slug: &str) -> String {
    let mut s = slug.trim().to_string();
    if s.is_empty() {
        return "/".to_string();
    }
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    s
}

pub async fn requester_from_session(session: &Session) -> Requester {
    let username: Option<String> = session.get("username").await.unwrap();
    match &username {
        Some(name) => {
            let users = crate::auth::load_users();
            let user = users.get(name);
            Requester {
                username: username.clone(),
                groups: user.and_then(|u| u.groups.clone()).unwrap_or_default(),
                userpv: user.map(|u| u.userpv.clone()).unwrap_or_default(),
            }
        }
        None => Requester::default(),
    }
}

/// Immediate parent namespace of a scope. "/a/b" -> "/a", "/a" -> "/", "/" -> "/".
pub fn parent_scope(scope: &str) -> String {
    let trimmed = scope.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(idx) => trimmed[..idx].to_string(),
    }
}

/// Whether `scope` is `ancestor` itself or lives beneath it (boundary-safe).
pub fn is_within(scope: &str, ancestor: &str) -> bool {
    if ancestor == "/" {
        return scope.starts_with('/');
    }
    scope == ancestor || (scope.starts_with(ancestor) && scope[ancestor.len()..].starts_with('/'))
}

pub fn target_matches(requester: &Requester, target: &str) -> bool {
    match target {
        "ALL" => true,
        "ALL_AUTH" => requester.username.is_some(),
        "ALL_UNAUTH" => requester.username.is_none(),
        t if t.starts_with("user:") => requester.username.as_deref() == Some(&t[5..]),
        group => requester.groups.iter().any(|g| g == group),
    }
}

/// Resolve the requester's EPV for a scope, §The Algorithm.
/// 1. sudo group bypass -> D
/// 2. userpv override (deepest matching scope wins, tie -> highest PV, cascades to children)
/// 3. walk scope -> root, first scope with a matching grant wins (highest PV among matches)
/// 4. root falls back to the config base/auth user permissions
/// 5. nothing matched -> N
pub fn resolve_epv(acl: &Acl, requester: &Requester, scope: &str) -> u8 {
    if requester.groups.iter().any(|g| g == "sudo") {
        return D;
    }

    let scope = scope_for_slug(scope);

    // userpv override
    let mut best: Option<(usize, u8)> = None;
    for (pv, scopes) in &requester.userpv {
        for s in scopes {
            let s = scope_for_slug(s);
            if is_within(&scope, &s) {
                let depth = s.matches('/').count();
                match best {
                    Some((bd, bpv)) if depth > bd || (depth == bd && *pv > bpv) => {
                        best = Some((depth, *pv));
                    }
                    None => best = Some((depth, *pv)),
                    _ => {}
                }
            }
        }
    }
    if let Some((_, pv)) = best {
        return pv;
    }

    // acl walk
    let mut current = scope.clone();
    loop {
        if let Some(pvs) = acl.get(&current) {
            let mut highest: Option<u8> = None;
            for (pv, targets) in pvs {
                if targets.iter().any(|t| target_matches(requester, t)) {
                    highest = Some(match highest {
                        Some(h) => h.max(*pv),
                        None => *pv,
                    });
                }
            }
            if let Some(h) = highest {
                return h;
            }
        }
        if current == "/" {
            break;
        }
        current = parent_scope(&current);
    }

    // root fallback from config (ALL always matches; ALL_AUTH only when logged in)
    let base = CFG.auth.base_user_permission.unwrap_or(N);
    let auth = CFG.auth.auth_user_permission.unwrap_or(N);
    if requester.username.is_some() {
        base.max(auth)
    } else {
        base
    }
}

pub fn can(epv: PV, required: PV) -> bool {
    epv >= required
}

pub fn can_access(acl: &Acl, requester: &Requester, scope: &str, required: u8) -> bool {
    can(resolve_epv(acl, requester, scope), required)
}
