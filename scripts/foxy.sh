#!/usr/bin/env bash
set -euo pipefail

REPO="${FOXY_REPO:-Wgmlgz/foxy-jumpscare}"
RELEASE_API_URL="https://api.github.com/repos/${REPO}/releases/latest"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"
RELEASE_JSON="$(curl -fsSL "${RELEASE_API_URL}")"

ASSET_NAMES=()
while IFS= read -r line; do
  ASSET_NAMES+=("${line}")
done < <(
  printf '%s\n' "${RELEASE_JSON}" | awk '
    /"assets":[[:space:]]*\[/ { in_assets=1; next }
    in_assets && /^[[:space:]]*\],?[[:space:]]*$/ { in_assets=0; next }
    in_assets && /"name":[[:space:]]*"/ {
      line=$0
      sub(/^[[:space:]]*"name":[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*,?[[:space:]]*$/, "", line)
      print line
    }
  '
)

ASSET_URLS=()
while IFS= read -r line; do
  ASSET_URLS+=("${line}")
done < <(
  printf '%s\n' "${RELEASE_JSON}" | awk '
    /"assets":[[:space:]]*\[/ { in_assets=1; next }
    in_assets && /^[[:space:]]*\],?[[:space:]]*$/ { in_assets=0; next }
    in_assets && /"browser_download_url":[[:space:]]*"/ {
      line=$0
      sub(/^[[:space:]]*"browser_download_url":[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*,?[[:space:]]*$/, "", line)
      print line
    }
  '
)

if [[ "${#ASSET_NAMES[@]}" -eq 0 || "${#ASSET_NAMES[@]}" -ne "${#ASSET_URLS[@]}" ]]; then
  echo "Failed to parse release assets from GitHub API" >&2
  exit 1
fi

pick_pattern=""
case "${OS}" in
  Darwin)
    if [[ "${ARCH}" == arm* || "${ARCH}" == "aarch64" ]]; then
      pick_pattern="_darwin_aarch64"
    else
      pick_pattern="_darwin_x64"
    fi
    ;;
  Linux)
    if [[ "${ARCH}" == arm* || "${ARCH}" == "aarch64" ]]; then
      pick_pattern="_linux_aarch64"
    else
      pick_pattern="_linux_x64"
    fi
    ;;
  *)
    echo "Unsupported OS for this script: ${OS}" >&2
    exit 1
    ;;
esac

ASSET_NAME=""
ASSET_URL=""
for i in "${!ASSET_NAMES[@]}"; do
  if [[ "${ASSET_NAMES[$i]}" == *"${pick_pattern}" ]]; then
    ASSET_NAME="${ASSET_NAMES[$i]}"
    ASSET_URL="${ASSET_URLS[$i]}"
    break
  fi
done

if [[ -z "${ASSET_NAME}" || -z "${ASSET_URL}" ]]; then
  echo "No matching portable binary asset found for ${OS}/${ARCH}" >&2
  exit 1
fi

if [[ "${FOXY_PRINT_ONLY:-0}" == "1" ]]; then
  echo "${ASSET_URL}"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
ASSET_PATH="${TMP_DIR}/${ASSET_NAME}"

curl -fL --retry 3 --retry-delay 1 -o "${ASSET_PATH}" "${ASSET_URL}"

chmod +x "${ASSET_PATH}"
"${ASSET_PATH}"
