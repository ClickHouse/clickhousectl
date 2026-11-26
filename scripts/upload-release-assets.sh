#!/usr/bin/env bash
# Run from the verified artifact directory, after SHA256SUMS has been checked.
set -euo pipefail

: "${S3_BUCKET:?}" "${S3_PREFIX:?}" "${GITHUB_REF_NAME:?}"
[[ "$GITHUB_REF_NAME" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]
work=$(mktemp -d)
trap 'rm -r -- "$work"' EXIT
missing=()
files=(./*.tar.gz SHA256SUMS)
object_key() {
    if [[ "$1" == SHA256SUMS ]]; then
        printf '%s/clickhousectl-%s-SHA256SUMS\n' "$S3_PREFIX" "$GITHUB_REF_NAME"
    else
        printf '%s/%s\n' "$S3_PREFIX" "${1#./}"
    fi
}

# Inspect the entire release before writing anything. Existing objects are
# accepted only when identical to the authorized local bytes.
for file in "${files[@]}"; do
    key=$(object_key "$file")
    if error=$(aws s3api head-object --bucket "$S3_BUCKET" --key "$key" 2>&1 >/dev/null); then
        aws s3 cp "s3://${S3_BUCKET}/${key}" "$work/existing" --only-show-errors
        if ! cmp -s -- "$file" "$work/existing"; then
            printf 'Existing S3 bytes differ: %s\n' "$key" >&2
            exit 1
        fi
        rm -- "$work/existing"
    elif [[ "$error" == *'(404)'* && "$error" == *'Not Found'* ]]; then
        missing+=("$file")
    else
        printf 'S3 head-object failed: %s\n' "$key" >&2
        exit 1
    fi
done

for file in "${missing[@]}"; do
    type=application/gzip
    [[ "$file" != SHA256SUMS ]] || type=text/plain
    key=$(object_key "$file")
    aws s3api put-object --bucket "$S3_BUCKET" --key "$key" \
        --body "$file" --content-type "$type" --if-none-match '*' >/dev/null
    aws s3 cp "s3://${S3_BUCKET}/${key}" "$work/readback" --only-show-errors
    if ! cmp -s -- "$file" "$work/readback"; then
        printf 'S3 read-back differs: %s\n' "$key" >&2
        exit 1
    fi
    rm -- "$work/readback"
done
