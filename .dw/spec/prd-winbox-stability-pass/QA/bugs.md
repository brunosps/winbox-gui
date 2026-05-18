# Bugs found in QA — winbox-stability-pass

**Round 1 (2026-05-17)**: zero bugs.

No regressions found in:
- cargo test (32 passing)
- node:test (13 passing)
- cargo clippy (zero warnings)
- cargo fmt (clean)
- manual smoke (mylinux launch + renderLaunchError invocations)

If future regressions surface, add an entry below per the template:

```
## NNN — short title

- **Severity**: low / medium / high
- **Found in**: cargo test path / npm test path / manual smoke step
- **Reproduction**: ...
- **Root cause**: ...
- **Fix**: linked commit
```
