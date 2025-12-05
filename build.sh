
#!/usr/bin/env bash
#  Authored in part with ChatGPT (GPT-5, OpenAI)
#  Reviewed and enhanced by GitHub Copilot (2025-10-28)
#  Final review by the human Jen Eleniel <jen.eleniel@bofa.com>

set -euo pipefail

# ==============================================================================
#  SVELTEKIT + EXPRESS FULL BUILD & PACKAGING SCRIPT
#  Produces a self-contained "build/" folder deployable under Node.js
#  Includes checksum manifest and compressed archive.
# ==============================================================================


BUILD_DIR="build"
DIST_DIR=".svelte-kit"
TMP_DIR=".build-tmp"
ARCHIVE_NAME="vanopticon_heimdall_deploy.tar.gz"

# --- TOOLING CHECKS ----------------------------------------------------------
REQUIRED_TOOLS=(jq pnpm pnpx npm sha256sum tar)
for tool in "${REQUIRED_TOOLS[@]}"; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "❌ Required tool '$tool' is not installed or not in PATH. Aborting."
    exit 2
  fi
done


# --- ACCIDENTAL OVERWRITE CHECK ----------------------------------------------
if [[ -f "$ARCHIVE_NAME" ]]; then
  echo "⚠ Archive $ARCHIVE_NAME already exists. Overwrite? [y/N]"
  read -r reply
  if [[ ! "$reply" =~ ^[Yy]$ ]]; then
    echo "Aborting build to avoid overwriting existing archive."
    exit 3
  fi
fi

echo "▶ Cleaning previous build artifacts..."
rm -rf "$BUILD_DIR" "$TMP_DIR" "$ARCHIVE_NAME"
mkdir -p "$TMP_DIR"

# --- DEPENDENCY INSTALL -------------------------------------------------------
echo "▶ Installing dependencies (including dev)..."
pnpm install --frozen-lockfile

# --- BUILD STAGE --------------------------------------------------------------
echo "▶ Running SvelteKit build..."
pnpm run build

# --- VERIFY ADAPTER OUTPUT ----------------------------------------------------
if [[ ! -d "$BUILD_DIR" && -d "$DIST_DIR/output" ]]; then
  echo "⚠ Adapter output not found in $BUILD_DIR; copying from $DIST_DIR/output..."
  cp -r "$DIST_DIR/output" "$BUILD_DIR"
fi

if [[ ! -d "$BUILD_DIR" ]]; then
  echo "❌ Build output missing. Ensure svelte.config.js uses '@sveltejs/adapter-node'."
  exit 1
fi

# --- CREATE PRODUCTION PACKAGE.JSON ------------------------------------------
echo "▶ Generating production package.json..."
jq 'del(.devDependencies)
    | del(.scripts.prepare)
    | .scripts = { "start": "node server.js" }' \
    package.json > "$TMP_DIR/package.json"

# --- COPY ARTIFACTS -----------------------------------------------------------
echo "▶ Copying runtime files..."
cp "$TMP_DIR/package.json" "$BUILD_DIR/package.json"

if [[ -d "static" ]]; then
  cp -r static "$BUILD_DIR/"
fi

if [[ -f "server.js" ]]; then
  cp "server.js" "$BUILD_DIR/"
elif [[ -f "src/server.ts" ]]; then
  echo "Compiling custom Express server (server.ts)..."
  pnpx tsc src/server.ts --outDir "$BUILD_DIR" --esModuleInterop
fi

if [[ -f ".env.production" ]]; then
  echo "▶ Copying .env.production..."
  cp .env.production "$BUILD_DIR/.env"
fi


# --- INSTALL PRODUCTION DEPENDENCIES -----------------------------------------
echo "▶ Installing production dependencies..."
pushd "$BUILD_DIR" >/dev/null
pnpm install --prod --frozen-lockfile --ignore-scripts --no-audit --no-fund
popd >/dev/null

# --- GENERATE CHECKSUMS ------------------------------------------------------
echo "▶ Generating SHA-256 checksums..."
pushd "$BUILD_DIR" >/dev/null
find . -type f ! -name "checksums.sha256" -exec sha256sum "{}" + | sort > checksums.sha256
popd >/dev/null

# --- PACKAGE -----------------------------------------------------------------
echo "▶ Creating deployment archive..."
tar -czf "$ARCHIVE_NAME" -C "$BUILD_DIR" .

# --- CLEANUP -----------------------------------------------------------------
echo "▶ Cleaning temporary files..."
rm -rf "$TMP_DIR"

# --- SUMMARY -----------------------------------------------------------------
echo
echo "✅ Build complete."
echo "  • Ready-to-run package:     $BUILD_DIR/"
echo "  • Checksum manifest:        $BUILD_DIR/checksums.sha256"
echo "  • Compressed deployment:    $ARCHIVE_NAME"
echo
echo "To verify after deployment:"
echo "  cd /path/to/deploy && sha256sum -c checksums.sha256"
echo
echo "To deploy manually:"
echo "  scp $ARCHIVE_NAME user@server:/opt/app/"
echo "  ssh user@server 'cd /opt/app && tar -xzf $ARCHIVE_NAME && node server.js'"
echo

exit 0
