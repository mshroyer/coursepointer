#!/bin/sh

# Publish the site to S3/CloudFront
#
# Vite puts cache-busting hashes in items in the assets/ directory, so we can
# set a separate, more aggressive caching policy for those.

set -e

BUCKET="s3://coursepointer.app"
DISTRIBUTION="E15WM16D97A4F9"

cd "$(dirname "$0")/../web"

pnpm build

aws s3 sync ./dist/ $BUCKET \
  --exclude "assets/*" \
  --cache-control "public, max-age=14400, must-revalidate"

# Cache hashed assets for 30 days:
aws s3 sync ./dist/assets/ $BUCKET/assets \
  --cache-control "public, max-age=2592000, immutable"

aws cloudfront create-invalidation \
    --distribution-id "$DISTRIBUTION" \
    --paths "/index.html" "/"
