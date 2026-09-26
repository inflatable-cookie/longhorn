let bun_args = ["scripts/release-bump.ts"];
for arg in args {
    bun_args.push(arg);
}

let result = process::run("bun", bun_args);
if !result["success"] {
    throw(result["stderr"]);
}
if result["stdout"] != "" {
    log(result["stdout"]);
}
