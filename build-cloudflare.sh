#!/bin/bash
set -e

# Cloudflare Pages Build Script
# NOTE: Cloudflare Pages is designed for static sites and edge functions.
# This Rust API service should be deployed to a container platform instead.
# See README.md for proper deployment instructions.

echo "🚀 Building Label Verify HW for Cloudflare Pages"
echo "================================================"
echo ""

# Check if this is actually a Cloudflare Pages build
if [ -n "$CF_PAGES" ]; then
    echo "⚠️  WARNING: This is a Rust API service with database dependencies."
    echo "⚠️  Cloudflare Pages is designed for static sites and edge functions."
    echo "⚠️  This service should be deployed to a container platform like:"
    echo "    - Fly.io"
    echo "    - Railway"
    echo "    - Render"
    echo "    - AWS ECS/Fargate"
    echo "    - Google Cloud Run"
    echo ""
    echo "For now, creating a placeholder build..."
    echo ""
fi

# Create output directory for Cloudflare Pages
mkdir -p dist

# Create a placeholder index.html explaining the deployment issue
cat > dist/index.html <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Label Verify HW - Deployment Notice</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .notice {
            background: white;
            border-left: 4px solid #ff6b35;
            padding: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 { color: #333; margin-top: 0; }
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: monospace;
        }
        .warning { color: #ff6b35; font-weight: bold; }
        ul { line-height: 1.8; }
    </style>
</head>
<body>
    <div class="notice">
        <h1>⚠️ Deployment Configuration Notice</h1>

        <p class="warning">This Rust API service cannot run on Cloudflare Pages.</p>

        <p>
            <strong>Label Verify HW</strong> is a backend API service built with:
        </p>
        <ul>
            <li>Rust + Axum web framework</li>
            <li>PostgreSQL database</li>
            <li>Redis queue</li>
            <li>Cloudflare R2 storage</li>
            <li>Cloudflare Workers AI</li>
        </ul>

        <h2>Recommended Deployment Platforms:</h2>
        <ul>
            <li><strong>Fly.io</strong> - Excellent Rust support, global deployment</li>
            <li><strong>Railway</strong> - Easy setup, includes PostgreSQL & Redis</li>
            <li><strong>Render</strong> - Free tier available, Docker support</li>
            <li><strong>AWS ECS/Fargate</strong> - Enterprise-grade container platform</li>
            <li><strong>Google Cloud Run</strong> - Serverless containers</li>
            <li><strong>VPS with Docker</strong> - Use included docker-compose.yml</li>
        </ul>

        <h2>Quick Deploy with Docker:</h2>
        <pre><code>cp .env.prod.example .env.prod
# Edit .env.prod with your values
./deploy.sh deploy</code></pre>

        <p>
            For detailed deployment instructions, see the
            <a href="https://github.com/aerocristobal/label-verify-hw">GitHub repository</a>.
        </p>
    </div>
</body>
</html>
EOF

echo "✅ Placeholder build created in dist/"
echo ""
echo "⚠️  To properly deploy this service:"
echo "   1. Choose a container platform (Fly.io, Railway, Render, etc.)"
echo "   2. Configure environment variables"
echo "   3. Deploy using Docker or the platform's CLI"
echo ""
echo "   See README.md for detailed instructions."
