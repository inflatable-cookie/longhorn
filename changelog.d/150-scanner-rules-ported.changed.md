The retired `scan attention-markers` and `scan generated-in-src` rules are ported
as `effigy check:markers` and `effigy check:generated-sources`. Each rule has a
synthetic fixture test, and both ports matched the stable scanners on one
fixture that covers every rule. `check:markers` fails on its critical tier
(`BUG`, `remove before release`). `check:generated-sources` covers the generated
header, source maps, minified assets, generated file names and generated path
components over the scanner's source roots, with the bindings output and
generator sources allowed.
