#!/bin/bash
set -e

FORMULA="Formula/clipzero.rb"
REPO="jn3ff/clipzero"

# Get current version from formula
CURRENT_VERSION=$(grep -E '^\s*version' "$FORMULA" | sed -E 's/.*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')

if [[ -z "$CURRENT_VERSION" ]]; then
    echo "Error: Could not parse current version from $FORMULA"
    exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

case "$1" in
    patch)
        PATCH=$((PATCH + 1))
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    *)
        echo "Usage: $0 <patch|minor|major>"
        echo "Current version: $CURRENT_VERSION"
        exit 1
        ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
NEW_TAG="v${NEW_VERSION}"

echo "Upgrading: $CURRENT_VERSION -> $NEW_VERSION"

# Create and push tag
echo "Creating tag $NEW_TAG..."
git tag "$NEW_TAG"
git push origin "$NEW_TAG"

# Wait a moment for GitHub to process the tag
echo "Waiting for GitHub to process tag..."
sleep 2

# Get SHA256 of new tarball
TARBALL_URL="https://github.com/${REPO}/archive/refs/tags/${NEW_TAG}.tar.gz"
echo "Fetching SHA256 from $TARBALL_URL..."
SHA256=$(curl -sL "$TARBALL_URL" | shasum -a 256 | awk '{print $1}')

if [[ -z "$SHA256" || ${#SHA256} -ne 64 ]]; then
    echo "Error: Failed to get valid SHA256"
    exit 1
fi

echo "SHA256: $SHA256"

# Update formula
echo "Updating $FORMULA..."
sed -i '' "s|url \"https://github.com/${REPO}/archive/refs/tags/.*\.tar\.gz\"|url \"${TARBALL_URL}\"|" "$FORMULA"
sed -i '' "s|version \".*\"|version \"${NEW_VERSION}\"|" "$FORMULA"
sed -i '' "s|sha256 \".*\"|sha256 \"${SHA256}\"|" "$FORMULA"

# Commit and push
echo "Committing and pushing..."
git add "$FORMULA"
git commit -m "bump to $NEW_TAG"
git push origin main

echo "Done! Released $NEW_TAG"
echo "Users can now run: brew upgrade clipzero"
