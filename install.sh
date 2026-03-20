#!/bin/bash
set -e

VERSION="v0.1.0"

echo ""
echo "  Phantom Engine Installer"
echo "  ========================"
echo ""

DOCKER=$(which docker 2>/dev/null || true)

if [ -z "$DOCKER" ]; then
    echo "Error: Docker is not installed."
    echo "Install Docker first: https://docs.docker.com/get-docker/"
    exit 1
fi

echo "Pulling polymit/phantom:${VERSION}..."
$DOCKER pull polymit/phantom:${VERSION}

echo ""
echo "Done! Run Phantom Engine with:"
echo ""
echo "  docker run -d -p 8080:8080 -e PHANTOM_API_KEYS=your-key polymit/phantom:${VERSION}"
echo ""
