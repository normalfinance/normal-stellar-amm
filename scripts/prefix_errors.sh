#!/bin/bash

# Accept path to index.ts (or default to ./index.ts)
FILE="${1:-index.ts}"
TMP_FILE="${FILE}.tmp"

>"$TMP_FILE"

inside_block=false
buffer=""
prefix=""
export_line=""

while IFS= read -r line || [[ -n "$line" ]]; do
    if $inside_block; then
        buffer+="$line"$'\n'

        # Extract first Error-like word from comment
        if [[ -z "$prefix" && "$line" =~ \* ]]; then
            prefix=$(echo "$line" | grep -oE '\b[A-Za-z0-9_]+Error\b' | head -n1)
        fi

        # Detect end of block by bare `}` (but don't worry if this logic fails—we’ll add one later)
        if [[ "$line" =~ ^\} ]]; then
            if [[ -n "$prefix" ]]; then
                buffer=$(echo "$buffer" | sed "1s/export const Errors =/export const ${prefix} =/")
            fi
            echo "$buffer" >>"$TMP_FILE"

            # Reset
            inside_block=false
            buffer=""
            prefix=""
        fi

        continue
    fi

    # Detect start of Errors block
    if [[ "$line" =~ ^export\ const\ Errors\ *=\ *\{ ]]; then
        inside_block=true
        buffer="$line"$'\n'
        continue
    fi

    # Just write any other line
    echo "$line" >>"$TMP_FILE"
done <"$FILE"

# ✅ Ensure file ends with `}` if it's missing
if ! tail -n 1 "$TMP_FILE" | grep -q '^\}'; then
    echo "}" >>"$TMP_FILE"
    echo "🛠️  Added missing closing brace at end of $FILE"
fi

mv "$TMP_FILE" "$FILE"

echo "✅ Finished renaming Errors blocks in $FILE"
