#!/bin/bash

# Setup script for TA-Lib source code
# This script downloads and prepares TA-Lib for compilation with Rust bindings

set -e

TALIB_VERSION="0.4.0"
TALIB_URL="https://sourceforge.net/projects/ta-lib/files/ta-lib/${TALIB_VERSION}/ta-lib-${TALIB_VERSION}-src.tar.gz"
TALIB_DIR="ta-lib"

echo "Setting up TA-Lib ${TALIB_VERSION} for Rust bindings..."

# Check if TA-Lib directory already exists
if [ -d "$TALIB_DIR" ]; then
    echo "TA-Lib directory already exists. Removing..."
    rm -rf "$TALIB_DIR"
fi

# Download TA-Lib source
echo "Downloading TA-Lib source..."
curl -L "$TALIB_URL" -o "ta-lib-${TALIB_VERSION}-src.tar.gz"

# Extract the archive
echo "Extracting TA-Lib source..."
tar -xzf "ta-lib-${TALIB_VERSION}-src.tar.gz"

# The archive extracts to "ta-lib" directory, so we're already good
echo "TA-Lib extracted to ta-lib/ directory"

# Clean up the downloaded archive
rm "ta-lib-${TALIB_VERSION}-src.tar.gz"

echo "TA-Lib source prepared in $TALIB_DIR/"

# Optional: Build TA-Lib as a static library
echo "Would you like to build TA-Lib as a static library? (y/n)"
read -r response

if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    echo "Building TA-Lib..."
    cd "$TALIB_DIR"

    # Configure and build
    if [ ! -f "configure" ]; then
        echo "Running autoreconf..."
        autoreconf -fiv
    fi

    ./configure --prefix="$(pwd)" --enable-static --disable-shared
    make clean
    make
    make install

    cd ..
    echo "TA-Lib built successfully!"
    echo "Library files are in $TALIB_DIR/lib/"
    echo "Header files are in $TALIB_DIR/include/"
else
    echo "Skipping TA-Lib build. You can build it later if needed."
fi

echo ""
echo "Setup complete! TA-Lib is ready for use with Rust bindings."
echo ""
echo "Next steps:"
echo "1. Run 'cargo check' to verify the bindings compile"
echo "2. Run 'cargo test' to test the functionality"
echo "3. Run 'cargo bench --bench benchmark_ema' to benchmark against TA-Lib"
echo ""
echo "Note: If you get linking errors, you may need to:"
echo "- Install autotools: apt-get install autotools-dev autoconf libtool"
echo "- Set LD_LIBRARY_PATH or use static linking in build.rs"
