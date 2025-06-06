#!/bin/bash

# Path to the prefixing script
PREFIX_SCRIPT="$(dirname "$0")/prefix_errors.sh"

# Find all index.ts files directly inside any subfolder of packages
find ./packages -type f -name index.ts | while read -r file; do
  echo "🔧 Running prefix_errors.sh on $file"
  "$PREFIX_SCRIPT" "$file"
done

echo "✅ Done processing all packages."
