# Git Hooks

Install a local Git hook wrapper for this clone:

```sh
./.githooks/install.sh
```

The local `.git/hooks/pre-commit` wrapper calls the versioned `.githooks/pre-commit` script. Git LFS stays on the default `.git/hooks` path, so its hooks continue to work normally.
