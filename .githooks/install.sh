#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
versioned_hook="$repo_root/.githooks/pre-commit"
hook_path="$repo_root/.git/hooks/pre-commit"

if [ ! -f "$versioned_hook" ]; then
    printf 'missing versioned hook: %s\n' "$versioned_hook" >&2
    exit 1
fi

wrapper_contents=$(cat <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# historical-marine-weather-gui managed wrapper
repo_root=$(git rev-parse --show-toplevel)
exec "$repo_root/.githooks/pre-commit" "$@"
EOF
)

if [ -f "$hook_path" ]; then
    if ! grep -Fq 'historical-marine-weather-gui managed wrapper' "$hook_path" \
        && ! grep -Fq 'exec "$repo_root/.githooks/pre-commit" "$@"' "$hook_path"; then
        printf 'refusing to overwrite existing hook: %s\n' "$hook_path" >&2
        printf 'move it aside or merge it manually, then rerun %s\n' "$0" >&2
        exit 1
    fi
fi

printf '%s\n' "$wrapper_contents" > "$hook_path"
chmod +x "$versioned_hook" "$hook_path"

printf 'installed pre-commit wrapper at %s\n' "$hook_path"
