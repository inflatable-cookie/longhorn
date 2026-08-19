// Forwards the install target to the bun installer. Extra args after `--`
// reach Rhai tasks; they do not reach shell-string tasks, and the global
// `--repo` flag switches catalogs (so it cannot name the install target).

let forwarded = [];
for arg in args {
    if arg != "--" && arg != "--help" && arg != "-h" {
        forwarded.push(arg);
    }
}

if forwarded.len() == 0 {
    throw("usage: effigy agent-control:install-skill -- <git-repo>\nThe path is the install target. Do not use the global --repo flag; that switches catalogs.");
}

let command_args = ["scripts/install-agent-control-skill.ts"];
for arg in forwarded {
    command_args.push(arg);
}

// tee, not run: rhai print() is swallowed by the TUI, so the installer
// must write its "installed / no-op / refused" line itself.
let result = process::tee("bun", command_args);
if !result.success {
    throw(if result.stderr.len() > 0 { result.stderr.trim() } else { "install failed" });
}
