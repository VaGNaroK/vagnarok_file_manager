#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Uso: $0 <versao-semver> (ex: 0.1.0-alpha.2 ou v0.1.0-alpha.2)" >&2
  exit 1
fi

RAW_VERSION="$1"
VERSION="${RAW_VERSION#v}"
TAG="v${VERSION}"

if [[ ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Erro: versao '${VERSION}' nao segue SemVer valido." >&2
  exit 1
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}"

CURRENT_BRANCH="$(git branch --show-current)"
if [[ "${CURRENT_BRANCH}" != "main" ]]; then
  echo "Erro: release deve ser criada a partir da branch 'main' (atual: '${CURRENT_BRANCH:-detached}')." >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Erro: arvore git contem alteracoes nao commitadas. Faca commit ou stash antes." >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null; then
  echo "Erro: a tag '${TAG}' ja existe localmente." >&2
  exit 1
fi

if [[ ! -f "CHANGELOG.md" ]] || ! grep -Eq "^## \[${VERSION}\]" "CHANGELOG.md"; then
  echo "Erro: adicione a secao '## [${VERSION}] - YYYY-MM-DD' no CHANGELOG.md antes de cortar a release." >&2
  exit 1
fi

echo "==> Atualizando [workspace.package].version em Cargo.toml para ${VERSION}..."
python3 - "${VERSION}" << 'PY'
import re
import sys

new_version = sys.argv[1]
with open("Cargo.toml", "r", encoding="utf-8") as f:
    lines = f.readlines()

in_workspace_pkg = False
updated = False
out = []
for line in lines:
    stripped = line.strip()
    if stripped.startswith("["):
        in_workspace_pkg = stripped == "[workspace.package]"
    elif in_workspace_pkg and not updated and re.match(r'^version\s*=\s*"[^"]+"\s*$', stripped):
        line = re.sub(r'"[^"]+"', f'"{new_version}"', line, count=1)
        updated = True
    out.append(line)

if not updated:
    print("Erro: campo version nao encontrado em [workspace.package] no Cargo.toml.", file=sys.stderr)
    sys.exit(1)

with open("Cargo.toml", "w", encoding="utf-8") as f:
    f.writelines(out)
PY

echo "==> Validando build e testes do workspace (e atualizando Cargo.lock)..."
cargo check --workspace --all-targets
cargo test --workspace

echo "==> Criando commit e tag anotada ${TAG}..."
git add Cargo.toml Cargo.lock
if ! git diff --cached --quiet; then
  git commit -m "chore(release): prepare ${TAG}"
fi
git tag -a "${TAG}" -m "Release ${TAG}"

echo ""
echo "Release ${TAG} preparada localmente!"
echo "Proximo passo para disparar o workflow de pacotes no GitHub Actions:"
echo "  git push origin main ${TAG}"
